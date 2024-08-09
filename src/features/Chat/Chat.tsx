import React, { useMemo } from "react";
// import { useEventBusForChat } from "../hooks/useEventBusForChat";
import type { Config } from "../../features/Config/reducer";
import { CodeChatModel, SystemPrompts } from "../../services/refact";
import { Chat as ChatComponent } from "../../components/Chat";
import {
  useGetCapsQuery,
  useGetPromptsQuery,
  useGetToolsQuery,
  useGetCommandCompletionQuery,
  useGetCommandPreviewQuery,
} from "../../app/hooks";
import { useDebounceCallback } from "usehooks-ts";
import { useAppDispatch, useAppSelector } from "../../app/hooks";
import {
  getSelectedSystemPrompt,
  setSystemPrompt,
  setUseTools,
} from "./chatThread";
import { getErrorMessage } from "../Errors/errorsSlice";

export type ChatProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
  style?: React.CSSProperties;
  backFromChat: () => void;
};

export const Chat: React.FC<ChatProps> = ({
  style,
  // askQuestion,
  // clearError,
  // setChatModel,
  // stopStreaming,
  backFromChat,
  // openChatInNewTab,
  // sendToSideBar,
  // handleNewFileClick,
  // handlePasteDiffClick,
  // hasContextFile,
  // requestCommandsCompletion,
  // requestPreviewFiles,
  // setSelectedCommand,
  // removePreviewFileByName,
  // retryQuestion,
  // maybeRequestCaps,
  // startNewChat,
  // setSelectedSystemPrompt,
  // setUseTools,
  // enableSend,
  // openSettings,
  host,
  tabbed,
  // state,
}) => {
  const error = useAppSelector(getErrorMessage);
  const capsRequest = useGetCapsQuery(undefined, { skip: !!error });
  const chatModel = useAppSelector((state) => state.chat.thread.model);

  // TODO: these could be lower in the component tree
  const promptsRequest = useGetPromptsQuery(undefined, { skip: !!error });
  const selectedSystemPrompt = useAppSelector(getSelectedSystemPrompt);
  const dispatch = useAppDispatch();
  const onSetSelectedSystemPrompt = (prompt: SystemPrompts) =>
    dispatch(setSystemPrompt(prompt));

  const useTools = useAppSelector((state) => state.chat.use_tools);
  const onSetUseTools = (value: boolean) => dispatch(setUseTools(value));
  const messages = useAppSelector((state) => state.chat.thread.messages);

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

  const sendToSideBar = () => {
    // TODO:
  };

  // const onSetSelectedSystemPrompt = useCallback(
  //   (key: string) => {
  //     if (!promptsRequest.data) return;
  //     if (!(key in promptsRequest.data)) return;
  //     if (key === "default") {
  //       setSelectedSystemPrompt("");
  //     } else {
  //       setSelectedSystemPrompt(key);
  //     }
  //   },
  //   [promptsRequest.data, setSelectedSystemPrompt],
  // );

  const maybeSendToSideBar =
    host === "vscode" && tabbed ? sendToSideBar : undefined;

  const canUseTools = useMemo(() => {
    if (!capsRequest.data) return false;
    if (!toolsRequest.data) return false;
    if (toolsRequest.data.length === 0) return false;
    const modelName = chatModel || capsRequest.data.code_chat_default_model;

    if (!(modelName in capsRequest.data.code_chat_models)) return false;
    const model: CodeChatModel = capsRequest.data.code_chat_models[modelName];
    if ("supports_tools" in model && model.supports_tools) return true;
    return false;
  }, [capsRequest.data, toolsRequest.data, chatModel]);

  // can be a selector
  const unCalledTools = React.useMemo(() => {
    if (messages.length === 0) return false;
    const last = messages[messages.length - 1];
    if (last.role !== "assistant") return false;
    const maybeTools = last.tool_calls;
    if (maybeTools && maybeTools.length > 0) return true;
    return false;
  }, [messages]);

  return (
    <ChatComponent
      style={style}
      host={host}
      tabbed={tabbed}
      backFromChat={backFromChat}
      // openChatInNewTab={openChatInNewTab}
      // onStopStreaming={stopStreaming}
      // chat={state.chat}
      // error={state.error}
      // onClearError={clearError}
      // retryQuestion={retryQuestion}
      // isWaiting={state.waiting_for_response}
      // isStreaming={state.streaming}
      // onNewFileClick={handleNewFileClick}
      // onPasteClick={handlePasteDiffClick}
      // canPaste={state.active_file.can_paste}
      // preventSend={state.prevent_send}
      unCalledTools={unCalledTools}
      // enableSend={enableSend}
      // onAskQuestion={(question: string) =>
      //   askQuestion(question, promptsRequest.data, toolsRequest.data)
      // }
      // TODO: This could be moved lower in the component tree
      caps={{
        error: capsRequest.error ? "error fetching caps" : null,
        fetching: capsRequest.isFetching,
        default_cap: capsRequest.data?.code_chat_default_model ?? "",
        available_caps: capsRequest.data?.code_chat_models ?? {},
      }}
      commands={commandResult}
      // is this used anywhere?
      // hasContextFile={hasContextFile}
      requestCommandsCompletion={requestCommandsCompletion}
      maybeSendToSidebar={maybeSendToSideBar}
      // activeFile={state.active_file}
      filesInPreview={commandPreview}
      // selectedSnippet={state.selected_snippet}
      // removePreviewFileByName={removePreviewFileByName}
      // requestCaps={() => {
      //   console.log("requestCaps called");
      //   void capsRequest.refetch();
      // }}
      prompts={promptsRequest.data ?? {}}
      // onStartNewChat={startNewChat}
      // Could be lowered
      onSetSystemPrompt={onSetSelectedSystemPrompt}
      selectedSystemPrompt={selectedSystemPrompt}
      requestPreviewFiles={() => ({})}
      canUseTools={canUseTools}
      setUseTools={onSetUseTools}
      useTools={useTools}
      // openSettings={openSettings}
    />
  );
};
