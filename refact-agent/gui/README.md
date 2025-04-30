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

`tabbed` is true or false, default `false` used in vscode for when the chat is rendered in a tab

`dev` if dev is true then the component works as it would when `host` is set to web but can display the chat as it would in another host setting.

`lspUrl` is the url of the refact-lsp server. If not set, the component will try to connect to the server using the default url `/`.

`themeProps`?: object containing some styles for the chat component.

```ts
interface ThemeProps = {

  hasBackground: boolean = true;

  appearance: "inherit" | "light" | "dark" = "inherit";

  accentColor: "tomato" | "red" | "ruby" | "crimson" | "pink"| "plum" | "purple" | "violet" | "iris" | "indigo" | "blue" | "cyan" | "teal" | "jade" | "green" | "grass" | "brown" | "orange" | "sky" | "mint" | "lime" | "yellow" | "amber" | "gold" | "bronze" | "gray" = "indigo";

  grayColor: "gray" | "mauve" | "slate" | "sage" | "olive" | "sand" | "auto" = "auto";

  panelBackground: "solid" | "translucent" = "translucent";

  radius: "none" | "small" | "medium" | "large" | "full" = "medium";

  scaling: "90%" | "95%" | "100%" | "105%" | "110%" = "100%";

}
```

#### Events

type definitions for events that chat component emits and receives from the host are in `src/events/index.ts` and exported from `dist/events/index.js`

## How to run

install dependencies: `npm ci`
run [refact-lsp](https://github.com/smallcloudai/refact-lsp)
run `REFACT_LSP_URL="http://localhost:8001" npm run dev` and go to http://localhost:5173

### env vars

`REFACT_LSP_URL`: URL of the refact-lsp server default is http://localhost:8001

## How to build

`npm run build`

### env vars

`VITE_REFACT_LSP_URL`: optional prefix for the `/v1/caps` and `/v1/chat` urls (used when building for docker)

## How to build for docker

`VITE_REFACT_LSP_URL="/lsp" npm run build`
and copy the files `dist/chat/styles.css` and `dist/index.umd.cjs` over to `refact/self_hosting_machinery/webgui/static/assets`

## Integrating with IDE's

### Message sent between chat and host.

Events types are loosely guarded by using `type branding` on the events `type` property

```ts
/**
 * This message is sent from the chat component to the host when the chat is mounted and ready to receive messages.
 */
interface ReadyMessage extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.READY; // "chat_ready"
  payload: { id: string };
}

/**
 * This messages is sent from the host to the chat component to restore a previously saved chat.
 */
interface RestoreChat extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT; // = "restore_chat_from_history"
  payload: {
    id: string;
    chat: ChatThread & {
      messages: ChatThread["messages"] | [string, string][];
    };
    snippet?: Snippet;
  };
}

/**
 * The host sends this message to start a new chat thread.
 */
interface CreateNewChatThread extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.NEW_CHAT; // = "create_new_chat"
  payload?: { id: string; snippet: string };
}

/**
 * Chat sends this to the host when asking a question.
 */
interface QuestionFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION; // = "chat_question"
  payload: ChatThread;
}
/**
 * Response from the host to the question
 */
interface ResponseToChat extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE; // = "chat_response",
  payload: ChatResponse;
}

/**
 * This message is sent from the host to the chat when the lsp is done streaming it's response
 */
interface ChatDoneStreaming extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.DONE_STREAMING; // = "chat_done_streaming"
  payload: { id: string };
}

/**
 * Sent from the host to the chat when an error has happened while streaming the response
 */
interface ChatErrorStreaming extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.ERROR_STREAMING; // = "chat_error_streaming"
  payload: { id: string; message: string };
}
/**
 * Request for command completion from the lsp
 * trigger will be null if the user has selected a command at that point it should be fine to send the query to the lsp
 */
interface RequestAtCommandCompletion extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.REQUEST_AT_COMMAND_COMPLETION; // = "chat_request_at_command_completion"
  payload: {
    id: string;
    query: string;
    cursor: number;
    trigger: string | null;
    number?: number;
  };
}

/**
 * This message is sent from the host to the chat contains the result of command completion request
 */
interface ReceiveAtCommandCompletion extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.RECEIVE_AT_COMMAND_COMPLETION; // = "chat_receive_at_command_completion"
  payload: { id: string } & CommandCompletionResponse;
}

/**
 * This message is sent from the chat component to the host to request for command preview
 */
interface RequestAtCommandPreview extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.REQUEST_AT_COMMAND_PREVIEW; // = "chat_request_at_command_preview"
  payload: { id: string; query: string; cursor: number };
}

/**
 * This message is sent from the chat component to the host when the response to the question is received.
 */
interface SaveChatFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.SAVE_CHAT; // = "save_chat_to_history"
  payload: ChatThread;
}

/**
 * Tells chat to replace the current message list, this is  done before sending a question to the lsp.
 */
interface BackUpMessages extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.BACKUP_MESSAGES; // = "back_up_messages"
  payload: { id: string; messages: ChatMessages };
}

/**
 * This message is sent from the chat component to the host when the user clicks stop while the response is streaming.
 */
interface StopStreamingFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.STOP_STREAMING; // = "chat_stop_streaming"
  payload: { id: string };
}

/**
 * chat requesting caps from the server.
 */
interface RequestCapsFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.REQUEST_CAPS; // = "chat_request_caps"
  payload: { id: string };
}

/**
 * This message is sent from the host to the chat when the server responds with caps.
 */
interface ChatReceiveCaps extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.RECEIVE_CAPS; // = "receive_caps"
  payload: { id: string; caps: CapsResponse };
}

/**
 * This message is sent from the host to the chat when the server responds with an error.
 */
interface ChatReceiveCapsError extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.RECEIVE_CAPS_ERROR; // = "receive_caps_error"
  payload: { id: string; message: string };
}

/**
 * This message is sent from the host to the chat with information about the current active file
 */
interface ActiveFileInfo extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.ACTIVE_FILE_INFO; // = "chat_active_file_info"
  payload: { id: string; name: string; can_paste: boolean };
}

/**
 * This message is sent from the host to the chat to set the selected snippet.
 */
interface ChatSetSelectedSnippet extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_SELECTED_SNIPPET; // = "chat_set_selected_command"
  payload: { id: string; snippet: { code: string; language: string } };
}

/**
 * This message is sent from the host telling chat to enable of disable the chat input.
 */
interface SetChatDisable extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_DISABLE_CHAT; // = "set_disable_chat"
  payload: { id: string; disable: boolean };
}

/**
 * Sets the default chat model for the chat.
 */
interface SetChatModel extends ActionToChat {
  type: EVENT_NAMES_TO_CHAT.SET_CHAT_MODEL; // = "chat_set_chat_model"
  payload: { id: string; model: string };
}

/**
 * This message is sent from the chat when the user clicks the `new file` button in a code example.
 */
interface NewFileFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.NEW_FILE; // = "chat_create_new_file"
  payload: { id: string; content: string };
}

/**
 * This message is sent from the chat when the user clicks the `paste` button in a code example.
 */
interface PasteDiffFromChat extends ActionFromChat {
  type: EVENT_NAMES_FROM_CHAT.PASTE_DIFF; // = "chat_paste_diff"
  payload: { id: string; content: string };
}
```

### Data types in the events

```ts
type ChatContextFile = {
  file_name: string;
  file_content: string;
  line1: number;
  line2: number;
};

interface ChatContextFileMessage extends BaseMessage {
  0: "context_file";
  1: ChatContextFile[];
}

interface UserMessage extends BaseMessage {
  0: "user";
  1: string;
}

interface AssistantMessage extends BaseMessage {
  0: "assistant";
  1: string;
}

type ChatMessage = UserMessage | AssistantMessage | ChatContextFileMessage;

type ChatMessages = ChatMessage[];

type ChatThread = {
  id: string;
  messages: ChatMessages;
  title?: string | undefined;
  model: string;
  attach_file?: boolean | undefined;
};

type ChatResponse =
  | { choices: ChatChoice[]; created: number; model: string; id: string }
  | ChatUserMessageResponse;

type ChatChoice = {
  delta: Delta;
  finish_reason: "stop" | "abort" | null;
  index: number;
};

type ChatUserMessageResponse = {
  id: string;
  role: "user" | "context_file";
  content: string;
};

type CapsResponse = {
  chat_default_model: string;
  chat_models: Record<string, CodeChatModel>;
};

type CodeCompletionModel = {
  default_scratchpad: string;
  n_ctx: number;
  similar_models: string[];
  supports_scratchpads: Record<string, Record<string, unknown>>;
};

type CommandCompletionResponse = {
  completions: string[];
  replace: Replace;
  is_cmd_executable: false;
};

interface Replace {
  0: number;
  1: number;
}
```
