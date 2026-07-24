#[cfg(windows)]
mod windows {
    use std::io::{self, Read, Write};
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use portable_pty::{CommandBuilder, PtySize, native_pty_system};

    const INITIAL_SIZE: PtySize = PtySize {
        rows: 25,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    };
    const RESIZED_SIZE: PtySize = PtySize {
        rows: 30,
        cols: 100,
        pixel_width: 0,
        pixel_height: 0,
    };
    const TEST_TIMEOUT: Duration = Duration::from_secs(20);
    const READ_INTERVAL: Duration = Duration::from_millis(50);
    const CURSOR_POSITION_REQUEST: &[u8] = b"\x1b[6n";
    const CURSOR_POSITION_REPORT: &[u8] = b"\x1b[1;1R";
    const ENTER_ALTERNATE_SCREEN: &[u8] = b"\x1b[?1049h";
    const LEAVE_ALTERNATE_SCREEN: &[u8] = b"\x1b[?1049l";
    const DOWN_ARROW: &[u8] = b"\x1b[B";
    const SUCCESS_MARKER: &[u8] = b"PowerShell TUI smoke: ok";

    #[test]
    fn powershell_runs_tui_in_conpty() {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(INITIAL_SIZE)
            .expect("create a Windows ConPTY");
        let command = powershell_command();
        let mut child = pair
            .slave
            .spawn_command(command)
            .expect("start PowerShell in ConPTY");
        drop(pair.slave);

        let (output_sender, output_receiver) = mpsc::channel();
        let mut reader = pair
            .master
            .try_clone_reader()
            .expect("clone the ConPTY output");
        let reader_thread = thread::spawn(move || read_output(&mut reader, output_sender));
        let mut writer = pair.master.take_writer().expect("open the ConPTY input");

        let deadline = Instant::now() + TEST_TIMEOUT;
        let mut output = Vec::new();
        let mut cursor_report_sent = false;
        let mut resize_sent = false;
        let mut navigation_sent = false;
        let mut quit_sent = false;
        let status = loop {
            receive_output(&output_receiver, &mut output);

            if contains(&output, CURSOR_POSITION_REQUEST) && !cursor_report_sent {
                writer
                    .write_all(CURSOR_POSITION_REPORT)
                    .and_then(|()| writer.flush())
                    .expect("report the inherited cursor position");
                cursor_report_sent = true;
            }
            if contains(&output, ENTER_ALTERNATE_SCREEN) && !resize_sent {
                pair.master
                    .resize(RESIZED_SIZE)
                    .expect("resize the Windows ConPTY");
                resize_sent = true;
            }
            if resize_sent && contains(&output, b"/200") && !navigation_sent {
                writer
                    .write_all(DOWN_ARROW)
                    .and_then(|()| writer.flush())
                    .expect("send a down-arrow key");
                navigation_sent = true;
            }
            if navigation_sent && contains(&output, b"2/200") && !quit_sent {
                writer
                    .write_all(b"q")
                    .and_then(|()| writer.flush())
                    .expect("send the quit key");
                quit_sent = true;
            }

            if let Some(status) = child.try_wait().expect("poll PowerShell") {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                panic!("PowerShell TUI timed out: {}", escape_output(&output));
            }
        };

        drop(writer);
        drop(pair.master);
        for result in output_receiver {
            output.extend(result.expect("read ConPTY output"));
        }
        reader_thread
            .join()
            .expect("join the ConPTY reader")
            .expect("finish reading ConPTY output");

        assert!(status.success(), "PowerShell exited with {status}");
        assert!(
            contains(&output, ENTER_ALTERNATE_SCREEN),
            "TUI did not enter the alternate screen: {}",
            escape_output(&output)
        );
        assert!(
            contains(&output, b"2/200"),
            "TUI did not process resize and navigation: {}",
            escape_output(&output)
        );
        assert!(
            contains(&output, LEAVE_ALTERNATE_SCREEN),
            "TUI did not restore the primary screen: {}",
            escape_output(&output)
        );
        assert!(
            contains(&output, SUCCESS_MARKER),
            "PowerShell script did not finish: {}",
            escape_output(&output)
        );
    }

    fn powershell_command() -> CommandBuilder {
        let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/test-windows-powershell-tui.ps1");
        let mut command = CommandBuilder::new("pwsh.exe");
        command.args(["-NoLogo", "-NoProfile", "-File"]);
        command.arg(script_path);
        command.arg("-KfindPath");
        command.arg(env!("CARGO_BIN_EXE_kfind"));
        command
    }

    fn read_output(
        reader: &mut dyn Read,
        sender: mpsc::Sender<io::Result<Vec<u8>>>,
    ) -> io::Result<()> {
        let mut buffer = [0; 4_096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(read) => {
                    if sender.send(Ok(buffer[..read].to_vec())).is_err() {
                        return Ok(());
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
                Err(error) => {
                    let message = io::Error::new(error.kind(), error.to_string());
                    let _ = sender.send(Err(message));
                    return Err(error);
                }
            }
        }
    }

    fn receive_output(receiver: &mpsc::Receiver<io::Result<Vec<u8>>>, output: &mut Vec<u8>) {
        match receiver.recv_timeout(READ_INTERVAL) {
            Ok(Ok(chunk)) => output.extend(chunk),
            Ok(Err(error)) => panic!("read ConPTY output: {error}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {}
        }
        while let Ok(result) = receiver.try_recv() {
            output.extend(result.expect("read ConPTY output"));
        }
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn escape_output(output: &[u8]) -> String {
        String::from_utf8_lossy(output).escape_debug().to_string()
    }
}
