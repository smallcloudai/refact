import React, { useCallback, useMemo } from "react";
import { useEventBusForChat } from "../hooks/useEventBusForChat";
import type { Config } from "../contexts/config-context";
import { CodeChatModel } from "../events";
import { Chat as ChatComponent } from "../components/Chat";
import {
  useGetCapsQuery,
  useGetPromptsQuery,
  useGetToolsQuery,
} from "../app/hooks";

type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  style?: React.CSSProperties;
} & ReturnType<typeof useEventBusForChat>;

export const Chat: React.FC<ChatProps> = ({
  style,
  askQuestion,
  clearError,
  setChatModel,
  stopStreaming,
  backFromChat,
  openChatInNewTab,
  sendToSideBar,
  handleNewFileClick,
  handlePasteDiffClick,
  hasContextFile,
  requestCommandsCompletion,
  requestPreviewFiles,
  setSelectedCommand,
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
  const toolsRequest = useGetToolsQuery(undefined);

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
    if (last[0] !== "assistant") return false;
    const maybeTools = last[2];
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
      onStopStreaming={stopStreaming}
      chat={state.chat}
      error={state.error}
      onClearError={clearError}
      retryQuestion={retryQuestion}
      isWaiting={state.waiting_for_response}
      isStreaming={state.streaming}
      onNewFileClick={handleNewFileClick}
      onPasteClick={handlePasteDiffClick}
      canPaste={state.active_file.can_paste}
      preventSend={state.prevent_send}
      unCalledTools={unCalledTools}
      enableSend={enableSend}
      onAskQuestion={(question: string) =>
        askQuestion(question, promptsRequest.data, toolsRequest.data)
      }
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
      commands={state.commands}
      hasContextFile={hasContextFile}
      requestCommandsCompletion={requestCommandsCompletion}
      setSelectedCommand={setSelectedCommand}
      maybeSendToSidebar={maybeSendToSideBar}
      activeFile={state.active_file}
      filesInPreview={state.files_in_preview}
      selectedSnippet={state.selected_snippet}
      removePreviewFileByName={removePreviewFileByName}
      requestCaps={maybeRequestCaps}
      // prompts={state.system_prompts.prompts}
      prompts={promptsRequest.data ?? {}}
      onStartNewChat={startNewChat}
      onSetSystemPrompt={onSetSelectedSystemPrompt}
      selectedSystemPrompt={state.selected_system_prompt}
      requestPreviewFiles={requestPreviewFiles}
      canUseTools={canUseTools}
      setUseTools={setUseTools}
      useTools={state.use_tools}
      openSettings={openSettings}
    />
  );
};
