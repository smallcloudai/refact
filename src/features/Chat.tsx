import React, { useMemo } from "react";
import { useEventBusForChat } from "../hooks/useEventBusForChat";
import type { Config } from "../contexts/config-context";
import { CodeChatModel } from "../events";
import { Chat as ChatComponent } from "../components/Chat";

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
  const maybeSendToSideBar =
    host === "vscode" && tabbed ? sendToSideBar : undefined;

  const canUseTools = useMemo(() => {
    if (state.tools === null || state.tools.length === 0) return false;
    const modelName = state.chat.model || state.caps.default_cap;
    if (!(modelName in state.caps.available_caps)) return false;
    const model: CodeChatModel = state.caps.available_caps[modelName];
    if ("supports_tools" in model && model.supports_tools) return true;
    return false;
  }, [
    state.tools,
    state.chat.model,
    state.caps.default_cap,
    state.caps.available_caps,
  ]);

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
      onAskQuestion={askQuestion}
      onSetChatModel={setChatModel}
      caps={state.caps}
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
      prompts={state.system_prompts.prompts}
      onStartNewChat={startNewChat}
      onSetSystemPrompt={setSelectedSystemPrompt}
      selectedSystemPrompt={state.selected_system_prompt}
      requestPreviewFiles={requestPreviewFiles}
      canUseTools={canUseTools}
      setUseTools={setUseTools}
      useTools={state.use_tools}
      openSettings={openSettings}
    />
  );
};
