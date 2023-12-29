/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_REFACT_LSP_URL?: string;
}
interface ImportMeta {
  readonly env: ImportMetaEnv;
}
