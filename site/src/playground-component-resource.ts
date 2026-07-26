import { DocumentLocale } from './app/i18n';
import { componentResourceRevision } from './kfind-wasm';

export enum ComponentResourceState {
  Checking = 'checking',
  Idle = 'idle',
  Needed = 'needed',
  Loading = 'loading',
  Ready = 'ready',
  Disabling = 'disabling',
  Disabled = 'disabled',
  Error = 'error',
}

export interface ComponentResourceStatus {
  readonly cached: boolean;
  readonly enabled: boolean;
  readonly message: string;
  readonly state: ComponentResourceState;
}

export interface ComponentResourceMessages {
  readonly initialResource: string;
  readonly resourceIdle: string;
  readonly resourceDisabled: string;
  readonly resourceDisableFailed: (error: string) => string;
  readonly resourceDisabling: string;
  readonly resourceLoading: string;
  readonly resourceNeeded: string;
  readonly resourceRestored: (byteLength: number, migrated: boolean) => string;
  readonly resourceStored: (byteLength: number, stored: boolean) => string;
  readonly resourceVerificationFailed: (error: string) => string;
}

export const componentResourceMessages: Readonly<
  Record<DocumentLocale, ComponentResourceMessages>
> = {
  [DocumentLocale.Korean]: {
    initialResource: `저장된 리소스 확인 중 · ${formatResourceRevision()}`,
    resourceIdle: `꺼짐 · 켜면 R2에서 35.4 MiB를 받습니다 · ${formatResourceRevision()}`,
    resourceDisabled: `설치됨 · 꺼짐 · ${formatResourceRevision()}`,
    resourceDisableFailed: (error) =>
      `형태 구성 요소 판정을 끄지 못했습니다 · ${error}`,
    resourceDisabling: '형태 구성 요소 판정을 끄는 중…',
    resourceLoading: 'R2에서 형태 구성 요소 판정 리소스를 불러오는 중…',
    resourceNeeded:
      '이 검색 질의의 smart 구조 판정에는 형태 구성 요소 판정이 필요합니다. 스위치를 켜 주세요.',
    resourceRestored: (byteLength, migrated) =>
      `${formatMebibytes(byteLength)} MiB ${migrated ? '저장소 복원 및 이전 완료' : '저장소 복원 완료'} · ${formatResourceRevision()}`,
    resourceStored: (byteLength, stored) =>
      stored
        ? `${formatMebibytes(byteLength)} MiB 로드·검증·저장 완료 · ${formatResourceRevision()}`
        : `${formatMebibytes(byteLength)} MiB 로드·검증 완료 · 저장소 미지원`,
    resourceVerificationFailed: (error) => `저장된 리소스 검증 실패 · ${error}`,
  },
  [DocumentLocale.English]: {
    initialResource: `Checking stored resource · ${formatResourceRevision()}`,
    resourceIdle: `Off · downloads 35.4 MiB from R2 when enabled · ${formatResourceRevision()}`,
    resourceDisabled: `Installed · off · ${formatResourceRevision()}`,
    resourceDisableFailed: (error) =>
      `Could not turn off morphological component verification · ${error}`,
    resourceDisabling: 'Turning off morphological component verification…',
    resourceLoading:
      'Loading the morphological component verification resource from R2…',
    resourceNeeded:
      'This query needs morphological component verification for its smart structural decision. Turn on the switch.',
    resourceRestored: (byteLength, migrated) =>
      `${formatMebibytes(byteLength)} MiB ${migrated ? 'restored and migrated' : 'restored'} · ${formatResourceRevision()}`,
    resourceStored: (byteLength, stored) =>
      stored
        ? `${formatMebibytes(byteLength)} MiB loaded, verified, and stored · ${formatResourceRevision()}`
        : `${formatMebibytes(byteLength)} MiB loaded and verified · storage unavailable`,
    resourceVerificationFailed: (error) =>
      `Stored resource validation failed · ${error}`,
  },
};

export function createInitialComponentResourceStatus(
  locale: DocumentLocale,
): ComponentResourceStatus {
  return {
    cached: false,
    enabled: false,
    state: ComponentResourceState.Checking,
    message: componentResourceMessages[locale].initialResource,
  };
}

function formatMebibytes(byteLength: number): string {
  return (byteLength / (1024 * 1024)).toFixed(1);
}

function formatResourceRevision(): string {
  return componentResourceRevision.slice(0, 12);
}
