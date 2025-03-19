import React, { useCallback, useMemo } from "react";
import {
  ChatMessages,
  isAssistantMessage,
  isChatContextFileMessage,
  isDiffMessage,
  isToolMessage,
  isUserMessage,
  UserMessage,
} from "../../services/refact";
import { UserInput } from "./UserInput";
import { ScrollArea, ScrollAreaWithAnchor } from "../ScrollArea";
import { Spinner } from "../Spinner";
import { Flex, Container, Button, Box } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";
import { PlainText } from "./PlainText";
import { useAppDispatch, useDiffFileReload } from "../../hooks";
import { useAppSelector } from "../../hooks";
import {
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectThread,
  selectThreadUsage,
} from "../../features/Chat/Thread/selectors";
import { takeWhile } from "../../utils";
import { GroupedDiffs } from "./DiffContent";
import { popBackTo } from "../../features/Pages/pagesSlice";
import { ChatLinks, UncommittedChangesWarning } from "../ChatLinks";
import { telemetryApi } from "../../services/refact/telemetry";
import { PlaceHolderText } from "./PlaceHolderText";
import { UsageCounter } from "../UsageCounter";
import { getConfirmationPauseStatus } from "../../features/ToolConfirmation/confirmationSlice";

export type ChatContentProps = {
  onRetry: (index: number, question: UserMessage["content"]) => void;
  onStopStreaming: () => void;
};

export const ChatContent: React.FC<ChatContentProps> = ({
  onStopStreaming,
  onRetry,
}) => {
  const dispatch = useAppDispatch();
  const messages = useAppSelector(selectMessages);
  const isStreaming = useAppSelector(selectIsStreaming);
  const thread = useAppSelector(selectThread);
  const threadUsage = useAppSelector(selectThreadUsage);
  const isConfig = thread.mode === "CONFIGURE";
  const isWaiting = useAppSelector(selectIsWaiting);
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();
  const integrationMeta = useAppSelector(selectIntegration);
  const isWaitingForConfirmation = useAppSelector(getConfirmationPauseStatus);

  const onRetryWrapper = (index: number, question: UserMessage["content"]) => {
    onRetry(index, question);
  };

  const handleReturnToConfigurationClick = useCallback(() => {
    // console.log(`[DEBUG]: going back to configuration page`);
    // TBD: should it be allowed to run in the background?
    onStopStreaming();
    dispatch(
      popBackTo({
        name: "integrations page",
        projectPath: thread.integration?.project,
        integrationName: thread.integration?.name,
        integrationPath: thread.integration?.path,
        wasOpenedThroughChat: true,
      }),
    );
  }, [
    onStopStreaming,
    dispatch,
    thread.integration?.project,
    thread.integration?.name,
    thread.integration?.path,
  ]);

  const handleManualStopStreamingClick = useCallback(() => {
    onStopStreaming();
    void sendTelemetryEvent({
      scope: `stopStreaming`,
      success: true,
      error_message: "",
    });
  }, [onStopStreaming, sendTelemetryEvent]);

  const shouldConfigButtonBeVisible = useMemo(() => {
    return isConfig && !integrationMeta?.path?.includes("project_summary");
  }, [isConfig, integrationMeta?.path]);

  // Dedicated hook for handling file reloads
  useDiffFileReload();

  return (
    <ScrollAreaWithAnchor.ScrollArea
      style={{ flexGrow: 1, height: "auto", position: "relative" }}
      scrollbars="vertical"
      type={isWaiting || isStreaming ? "auto" : "hover"}
      fullHeight
    >
      <Flex
        direction="column"
        className={styles.content}
        data-element="ChatContent"
        p="2"
        gap="1"
      >
        {messages.length === 0 && <PlaceHolderText />}
        {renderMessages(messages, onRetryWrapper, isWaiting)}
        <UncommittedChangesWarning />
        {threadUsage && messages.length > 0 && <UsageCounter />}
        <Container py="4">
          <Spinner
            spinning={(isStreaming || isWaiting) && !isWaitingForConfirmation}
          />
        </Container>
      </Flex>

      <Box
        style={{
          position: "absolute",
          bottom: 0,
          maxWidth: "100%", // TODO: make space for the down button
        }}
      >
        <ScrollArea scrollbars="horizontal">
          <Flex align="start" gap="3" pb="2">
            {isStreaming && (
              <Button
                // ml="auto"
                color="red"
                title="stop streaming"
                onClick={handleManualStopStreamingClick}
              >
                Stop
              </Button>
            )}
            {shouldConfigButtonBeVisible && (
              <Button
                // ml="auto"
                color="gray"
                title="Return to configuration page"
                onClick={handleReturnToConfigurationClick}
              >
                Return
              </Button>
            )}

            <ChatLinks />
          </Flex>
        </ScrollArea>
      </Box>
    </ScrollAreaWithAnchor.ScrollArea>
  );
};

ChatContent.displayName = "ChatContent";

function renderMessages(
  messages: ChatMessages,
  onRetry: (index: number, question: UserMessage["content"]) => void,
  waiting: boolean,
  memo: React.ReactNode[] = [],
  index = 0,
) {
  if (messages.length === 0) return memo;
  const [head, ...tail] = messages;
  if (head.role === "tool") {
    return renderMessages(tail, onRetry, waiting, memo, index + 1);
  }

  if (head.role === "plain_text") {
    const key = "plain-text-" + index;
    const nextMemo = [...memo, <PlainText key={key}>{head.content}</PlainText>];
    return renderMessages(tail, onRetry, waiting, nextMemo, index + 1);
  }

  if (head.role === "assistant") {
    const key = "assistant-input-" + index;
    const isLast = !tail.some(isAssistantMessage);
    const nextMemo = [
      ...memo,
      <AssistantInput
        key={key}
        message={head.content}
        reasoningContent={head.reasoning_content}
        toolCalls={head.tool_calls}
        isLast={isLast}
      />,
    ];

    return renderMessages(tail, onRetry, waiting, nextMemo, index + 1);
  }

  if (head.role === "user") {
    const key = "user-input-" + index;
    const isLastUserMessage = !tail.some(isUserMessage);
    const nextMemo = [
      ...memo,
      isLastUserMessage && (
        <ScrollAreaWithAnchor.ScrollAnchor
          key={`${key}-anchor`}
          behavior="smooth"
          block="start"
          // my="-2"
        />
      ),
      <UserInput onRetry={onRetry} key={key} messageIndex={index}>
        {head.content}
      </UserInput>,
    ];
    return renderMessages(tail, onRetry, waiting, nextMemo, index + 1);
  }

  if (isChatContextFileMessage(head)) {
    const key = "context-file-" + index;
    const nextMemo = [...memo, <ContextFiles key={key} files={head.content} />];
    return renderMessages(tail, onRetry, waiting, nextMemo, index + 1);
  }

  if (isDiffMessage(head)) {
    const restInTail = takeWhile(tail, (message) => {
      return isDiffMessage(message) || isToolMessage(message);
    });

    const nextTail = tail.slice(restInTail.length);
    const diffMessages = [head, ...restInTail.filter(isDiffMessage)];
    const key = "diffs-" + index;

    const nextMemo = [...memo, <GroupedDiffs key={key} diffs={diffMessages} />];

    return renderMessages(
      nextTail,
      onRetry,
      waiting,
      nextMemo,
      index + diffMessages.length,
    );
  }

  return renderMessages(tail, onRetry, waiting, memo, index + 1);
}
