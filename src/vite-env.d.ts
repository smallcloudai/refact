/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_REFACT_LSP_PORT?: string;
}
interface ImportMeta {
  readonly env: ImportMetaEnv;
}

type VersionInfo = { semver?: string; commit?: string } | undefined;
declare const __REFACT_CHAT_VERSION__: VersionInfo;
declare const __REFACT_LSP_PORT__: number | undefined;
interface Window {
  __REFACT_CHAT_VERSION__: VersionInfo;
  __REFACT_LSP_PORT__?: number;
}
