# Refact Chat Js

Shared component for chat with refact plugins and [refact docker](https://github.com/smallcloudai/refact).

## Usage

Add the bundled package to an html page and pass the selected element and options to `RefactChat.render` function.

```html
<body>
  <div id="refact-chat"></div>

  <script src="https://unpkg.com/refact-chat-js@0.0.1/dist/chat/index.umd.cjs"></script>

  <script>
    window.onload = function () {
      const root = document.getElementById("refact-chat");
      RefactChat.render(root, { host: "web" });
    };
  </script>
</body>
```

### API

#### `RefactChat.render(element, Options)`

- Element - the root element of the chat
- Options - the options to pass to the chat component

##### `Options`

`host` one of `[web, ide, vscode, jetbrains]`
when `host` is `web` the chat will be rendered in the browser and the events to and from chat will be handled by the side bar.
when `host` is `ide`, `vscode` or `jetbrains` events to and from the chat will be handled by the corresponding IDE or code editor via the `postMessage` API.
when ``

`tabbed` is true or false, default `false`

`dev` if dev is true then the component works as it would when `host` is set to web but can display the chat as it would in another host setting.

#### Events

type definitions for events that chat component emits and receives from the host are in `src/events/index.ts` and exported from `dist/events/index.js`

## How to run

install dependencies: `npm ci`
run [refact-lsp](https://github.com/smallcloudai/refact-lsp)
run `REFACT_LSP_URL="http://localhost:8001 npm dev` and go to localhost:5173

### env vars

`REFACT_LSP_URL`: URL of the refact-lsp server default is http://localhost:8001

## How to build

`npm run build`

### env vars

`VITE_REFACT_LSP_URL`: optional prefix for the `/v1/caps` and `/v1/chat` urls (use when building for docker)

## How to build for docker

`VITE_REFACT_LSP_URL="/lsp" npm run build -- --base=chat`
and copy the files over to `refact/self_hosting_machinery/webgui/static` renaming "dist/index.html" to "chat.html"
