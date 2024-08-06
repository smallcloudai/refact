import React, { useCallback, useMemo } from "react";
import { useEventBusForChat } from "../hooks/useEventBusForChat";
import type { Config } from "../app/hooks";
import { CodeChatModel } from "../events";
import { Chat as ChatComponent } from "../components/Chat";
import {
  useGetCapsQuery,
  useGetPromptsQuery,
  useGetToolsQuery,
  useGetCommandCompletionQuery,
  useGetCommandPreviewQuery,
} from "../app/hooks";
import { useDebounceCallback } from "usehooks-ts";
import {} from "../features/Chat2/chatThread";

type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  style?: React.CSSProperties;
} & ReturnType<typeof useEventBusForChat>;

export const Chat: React.FC<ChatProps> = ({
  style,
  // askQuestion,
  clearError,
  setChatModel,
  // stopStreaming,
  backFromChat,
  openChatInNewTab,
  sendToSideBar,
  handleNewFileClick,
  handlePasteDiffClick,
  hasContextFile,
  // requestCommandsCompletion,
  // requestPreviewFiles,
  // setSelectedCommand,
  removePreviewFileByName,
  retryQuestion,
  maybeRequestCaps,
  startNewChat,
  setSelectedSystemPrompt,
  setUseTools,
  enableSend,
  openSettings,
  host,
  tabbed,
  state,
}) => {
  const capsRequest = useGetCapsQuery(undefined);
  const promptsRequest = useGetPromptsQuery(undefined);

  // TODO: don't make this request if there are no caps
  const toolsRequest = useGetToolsQuery(!!capsRequest.data);
  // const chatRequest = useSendChatRequest();

  // commands should be a selector, and calling the hook ?
  const [command, setCommand] = React.useState<{
    query: string;
    cursor: number;
  }>({ query: "", cursor: 0 });

  // TODO: this could be put lower in the componet tree to prevent re-renders.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const requestCommandsCompletion = React.useCallback(
    useDebounceCallback(
      (query: string, cursor: number) => {
        setCommand({ query, cursor });
      },
      500,
      { leading: true, maxWait: 250 },
    ),
    [setCommand],
  );

  const commandResult = useGetCommandCompletionQuery(
    command.query,
    command.cursor,
    !!capsRequest.data,
  );

  const commandPreview = useGetCommandPreviewQuery(
    command.query,
    !!capsRequest.data,
  );

  const onSetSelectedSystemPrompt = useCallback(
    (key: string) => {
      if (!promptsRequest.data) return;
      if (!(key in promptsRequest.data)) return;
      if (key === "default") {
        setSelectedSystemPrompt("");
      } else {
        setSelectedSystemPrompt(key);
      }
    },
    [promptsRequest.data, setSelectedSystemPrompt],
  );

  const maybeSendToSideBar =
    host === "vscode" && tabbed ? sendToSideBar : undefined;

  const canUseTools = useMemo(() => {
    if (!capsRequest.data) return false;
    if (!toolsRequest.data) return false;
    if (toolsRequest.data.length === 0) return false;
    const modelName =
      state.chat.model || capsRequest.data.code_chat_default_model;

    if (!(modelName in capsRequest.data.code_chat_models)) return false;
    const model: CodeChatModel = capsRequest.data.code_chat_models[modelName];
    if ("supports_tools" in model && model.supports_tools) return true;
    return false;
  }, [capsRequest.data, toolsRequest.data, state.chat.model]);

  const unCalledTools = React.useMemo(() => {
    if (state.chat.messages.length === 0) return false;
    const last = state.chat.messages[state.chat.messages.length - 1];
    if (last.role !== "assistant") return false;
    const maybeTools = last.tool_calls;
    if (maybeTools && maybeTools.length > 0) return true;
    return false;
  }, [state.chat.messages]);

  return (
    <ChatComponent
      style={style}
      host={host}
      tabbed={tabbed}
      backFromChat={backFromChat}
      openChatInNewTab={openChatInNewTab}
      // onStopStreaming={stopStreaming}
      chat={state.chat}
      error={state.error}
      onClearError={clearError}
      retryQuestion={retryQuestion}
      // isWaiting={state.waiting_for_response}
      // isStreaming={state.streaming}
      onNewFileClick={handleNewFileClick}
      onPasteClick={handlePasteDiffClick}
      // canPaste={state.active_file.can_paste}
      preventSend={state.prevent_send}
      unCalledTools={unCalledTools}
      enableSend={enableSend}
      // onAskQuestion={(question: string) =>
      //   askQuestion(question, promptsRequest.data, toolsRequest.data)
      // }
      onSetChatModel={(value) => {
        const model =
          capsRequest.data?.code_completion_default_model === value
            ? ""
            : value;
        setChatModel(model);
      }}
      // TODO: This could be moved lower in the component tree
      caps={{
        error: capsRequest.error ? "error fetching caps" : null,
        fetching: capsRequest.isFetching,
        default_cap: capsRequest.data?.code_chat_default_model ?? "",
        available_caps: capsRequest.data?.code_chat_models ?? {},
      }}
      commands={commandResult}
      hasContextFile={hasContextFile}
      requestCommandsCompletion={requestCommandsCompletion}
      maybeSendToSidebar={maybeSendToSideBar}
      // activeFile={state.active_file}
      filesInPreview={commandPreview}
      // selectedSnippet={state.selected_snippet}
      removePreviewFileByName={removePreviewFileByName}
      requestCaps={maybeRequestCaps}
      prompts={promptsRequest.data ?? {}}
      onStartNewChat={startNewChat}
      onSetSystemPrompt={onSetSelectedSystemPrompt}
      selectedSystemPrompt={state.selected_system_prompt}
      requestPreviewFiles={() => ({})}
      canUseTools={canUseTools}
      setUseTools={setUseTools}
      useTools={state.use_tools}
      openSettings={openSettings}
    />
  );
};
