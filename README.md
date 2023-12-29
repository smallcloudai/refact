# Refact Chat Js

⚠️ This is a work in progress ⚠️

## How to run

install dependencies: `npm ci`
run [refact-lsp](https://github.com/smallcloudai/refact-lsp)
run `npm dev` and go to localhost:5173

### env vars

`REFACT_LSP_URL`: URL of the refact-lsp server default is http://localhost:8001

## How to build

`npm run build`

### env vars

`VITE_REFACT_LSP_URL`: optional prefix for the `/v1/caps` and `/v1/chat` urls (use when building for docker)

## How to build for docker

`VITE_REFACT_LSP_URL="/lsp" npm run build -- --base=chat`
and copy the files over to `refact/self_hosting_machinery/webgui/static` renaming "dist/index.html" to "chat.html"
