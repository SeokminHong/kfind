import { Select } from '@base-ui/react/select';
import { useEffect, useMemo, useState } from 'react';
import { useLocation } from 'react-router';

import { useDocumentTranslation } from '../i18n';
import { currentDocumentVersion, isVersionedSiteBuild } from '../site-build';

import * as styles from './version-selector.css';

interface DocumentVersion {
  readonly path: string;
  readonly prerelease: boolean;
  readonly version: string;
}

interface DocumentVersionManifest {
  readonly latest: string;
  readonly schemaVersion: 1;
  readonly versions: readonly DocumentVersion[];
}

const versionPattern =
  /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-rc\.[1-9][0-9]*)?$/u;

export function VersionSelector(): React.JSX.Element {
  const { t } = useDocumentTranslation();
  const location = useLocation();
  const [manifest, setManifest] = useState<DocumentVersionManifest>();

  useEffect(() => {
    const abortController = new AbortController();

    void loadVersionManifest(abortController.signal).then((value) => {
      if (value !== undefined) {
        setManifest(value);
      }
    });
    return () => {
      abortController.abort();
    };
  }, []);

  const versions = useMemo(() => {
    const published = manifest?.versions.map((entry) => entry.version) ?? [];
    return published.includes(currentDocumentVersion)
      ? published
      : [currentDocumentVersion, ...published];
  }, [manifest]);
  const cleanVersion = isVersionedSiteBuild()
    ? manifest?.latest
    : currentDocumentVersion;
  const options = versions.map((version) => ({
    label:
      cleanVersion !== undefined && version === cleanVersion
        ? t('common.version.latest', { version })
        : version,
    value: version,
  }));

  return (
    <Select.Root
      items={options}
      onValueChange={(version) => {
        if (version === null || version === currentDocumentVersion) {
          return;
        }
        globalThis.location.assign(
          versionHref(version, cleanVersion, {
            hash: location.hash,
            pathname: location.pathname,
            search: location.search,
          }),
        );
      }}
      value={currentDocumentVersion}
    >
      <Select.Trigger
        aria-label={t('common.version.aria')}
        className={styles.trigger}
      >
        <Select.Value className={styles.value} />
        <Select.Icon className={styles.icon}>▾</Select.Icon>
      </Select.Trigger>
      <Select.Portal>
        <Select.Positioner
          align="end"
          alignItemWithTrigger={false}
          className={styles.positioner}
          sideOffset={4}
        >
          <Select.Popup className={styles.popup}>
            <Select.List className={styles.list}>
              {options.map((option) => (
                <Select.Item
                  className={styles.item}
                  key={option.value}
                  value={option.value}
                >
                  <Select.ItemText>{option.label}</Select.ItemText>
                </Select.Item>
              ))}
            </Select.List>
          </Select.Popup>
        </Select.Positioner>
      </Select.Portal>
    </Select.Root>
  );
}

async function loadVersionManifest(
  signal: AbortSignal,
): Promise<DocumentVersionManifest | undefined> {
  try {
    const response = await fetch('/api/docs-versions', {
      headers: { Accept: 'application/json' },
      signal,
    });
    if (!response.ok) {
      return undefined;
    }
    const value: unknown = await response.json();
    return isDocumentVersionManifest(value) ? value : undefined;
  } catch (error: unknown) {
    if (error instanceof DOMException && error.name === 'AbortError') {
      return undefined;
    }
    return undefined;
  }
}

function isDocumentVersionManifest(
  value: unknown,
): value is DocumentVersionManifest {
  if (!isRecord(value) || value.schemaVersion !== 1) {
    return false;
  }
  if (
    typeof value.latest !== 'string' ||
    !versionPattern.test(value.latest) ||
    !Array.isArray(value.versions)
  ) {
    return false;
  }
  return value.versions.every(
    (entry) =>
      isRecord(entry) &&
      typeof entry.version === 'string' &&
      versionPattern.test(entry.version) &&
      typeof entry.prerelease === 'boolean' &&
      entry.prerelease === entry.version.includes('-rc.') &&
      entry.path === `/versions/${entry.version}`,
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function versionHref(
  version: string,
  cleanVersion: string | undefined,
  location: {
    readonly hash: string;
    readonly pathname: string;
    readonly search: string;
  },
): string {
  const path = location.pathname === '/' ? '/' : location.pathname;
  const prefix = version === cleanVersion ? '' : `/versions/${version}`;
  return `${prefix}${path}${location.search}${location.hash}`;
}
