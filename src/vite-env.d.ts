/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_REFACT_LSP_URL?: string;
}
interface ImportMeta {
  readonly env: ImportMetaEnv;
}

type VersionInfo = { semver?: string; commit?: string } | undefined;
declare const __REFACT_CHAT_VERSION__: VersionInfo;

interface Window {
  __REFACT_CHAT_VERSION__: VersionInfo;
}
