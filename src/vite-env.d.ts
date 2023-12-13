/// <reference types="vite/client" />

interface ImportMetaEnv {
    readonly VITE_REFACT_API_KEY: string
    // more env variables...
}

interface ImportMeta {
    readonly env: ImportMetaEnv
}