import { spawn } from 'node:child_process';

export async function runChromeProbe({
  executable,
  profileDirectory,
  target,
  timeoutMilliseconds,
}) {
  const launched = await launchChrome(executable, profileDirectory);
  let browser;
  let page;
  try {
    browser = await connectCdp(launched.webSocketUrl);
    const endpoint = new URL(launched.webSocketUrl);
    const targetResponse = await fetch(
      `http://${endpoint.host}/json/new?${encodeURIComponent(target)}`,
      { method: 'PUT' },
    );
    if (!targetResponse.ok) {
      throw new Error(
        `failed to create Chrome target: HTTP ${targetResponse.status}`,
      );
    }
    const targetDescription = await targetResponse.json();
    page = await connectCdp(targetDescription.webSocketDebuggerUrl);
    await page.send('Runtime.enable');
    return await waitForPublishedResult(page, timeoutMilliseconds);
  } catch (error) {
    throw new Error(
      `${error instanceof Error ? error.message : String(error)}\n${launched.stderr().trim()}`,
    );
  } finally {
    page?.close();
    await closeChrome(browser, launched.child);
  }
}

async function launchChrome(executable, profileDirectory) {
  const child = spawn(
    executable,
    [
      '--headless=new',
      '--disable-background-networking',
      '--disable-component-update',
      '--disable-default-apps',
      '--disable-extensions',
      '--disable-sync',
      '--enable-precise-memory-info',
      '--js-flags=--expose-gc',
      '--metrics-recording-only',
      '--no-first-run',
      '--no-service-autorun',
      '--password-store=basic',
      '--remote-debugging-address=127.0.0.1',
      '--remote-debugging-port=0',
      `--user-data-dir=${profileDirectory}`,
      'about:blank',
    ],
    { stdio: ['ignore', 'pipe', 'pipe'] },
  );
  child.stdout.resume();
  child.stderr.setEncoding('utf8');

  let stderr = '';
  const webSocketUrl = await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      reject(new Error(`Chrome DevTools endpoint timed out: ${stderr.trim()}`));
    }, 10_000);
    child.stderr.on('data', (chunk) => {
      stderr += chunk;
      const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) {
        clearTimeout(timeout);
        resolve(match[1]);
      }
    });
    child.once('error', (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.once('close', (code) => {
      clearTimeout(timeout);
      reject(new Error(`Chrome exited before DevTools was ready: ${code}`));
    });
  });
  return { child, stderr: () => stderr, webSocketUrl };
}

async function waitForPublishedResult(page, timeoutMilliseconds) {
  const deadline = Date.now() + timeoutMilliseconds;
  while (Date.now() < deadline) {
    try {
      const evaluation = await page.send('Runtime.evaluate', {
        expression:
          'JSON.stringify({error: document.documentElement.dataset.error ?? null, result: document.documentElement.dataset.result ?? null})',
        returnByValue: true,
      });
      const published = JSON.parse(evaluation.result.value);
      if (published.error || published.result) {
        return published;
      }
    } catch {
      // Navigation may replace the execution context between polling calls.
    }
    await delay(10);
  }
  throw new Error('browser benchmark result timed out');
}

async function closeChrome(browser, child) {
  if (browser) {
    try {
      await browser.send('Browser.close');
    } catch {
      // Fall through to terminating the process below.
    }
    browser.close();
  }
  if (child.exitCode !== null) {
    return;
  }
  const exited = await Promise.race([
    new Promise((resolve) => child.once('close', () => resolve(true))),
    delay(5_000).then(() => false),
  ]);
  if (!exited && child.exitCode === null) {
    child.kill('SIGKILL');
  }
}

function connectCdp(url) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(url);
    let nextId = 1;
    const pending = new Map();
    socket.addEventListener('message', (event) => {
      const message = JSON.parse(String(event.data));
      if (!message.id) {
        return;
      }
      const call = pending.get(message.id);
      if (!call) {
        return;
      }
      pending.delete(message.id);
      if (message.error) {
        call.reject(new Error(message.error.message));
      } else {
        call.resolve(message.result);
      }
    });
    socket.addEventListener('close', () => {
      for (const call of pending.values()) {
        call.reject(new Error('CDP WebSocket closed'));
      }
      pending.clear();
    });
    socket.addEventListener(
      'open',
      () =>
        resolve({
          close: () => socket.close(),
          send: (method, params = {}) => {
            const id = nextId;
            nextId += 1;
            return new Promise((resolveCall, rejectCall) => {
              pending.set(id, { reject: rejectCall, resolve: resolveCall });
              socket.send(JSON.stringify({ id, method, params }));
            });
          },
        }),
      { once: true },
    );
    socket.addEventListener(
      'error',
      () => reject(new Error('CDP WebSocket failed')),
      { once: true },
    );
  });
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
