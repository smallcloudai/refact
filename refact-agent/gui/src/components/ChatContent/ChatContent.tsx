import React, { useCallback, useMemo } from "react";
import {
  ChatMessages,
  isChatContextFileMessage,
  isDiffMessage,
  isToolMessage,
  isUserMessage,
  UserMessage,
} from "../../services/refact";
import { UserInput } from "./UserInput";
import { ScrollArea, ScrollAreaWithAnchor } from "../ScrollArea";
import { Flex, Container, Button, Box } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";

import { PlainText } from "./PlainText";
import { MessageUsageInfo } from "./MessageUsageInfo";
import { useAppDispatch, useDiffFileReload } from "../../hooks";
import { useAppSelector } from "../../hooks";
import {
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectQueuedMessages,
  selectThread,
} from "../../features/Chat/Thread/selectors";
import { takeWhile } from "../../utils";
import { GroupedDiffs } from "./DiffContent";
import { popBackTo } from "../../features/Pages/pagesSlice";
import { ChatLinks, UncommittedChangesWarning } from "../ChatLinks";
import { telemetryApi } from "../../services/refact/telemetry";
import { PlaceHolderText } from "./PlaceHolderText";

import { QueuedMessage } from "./QueuedMessage";
import { selectThreadConfirmation, selectThreadPause } from "../../features/Chat";

import { LogoAnimation } from "../LogoAnimation/LogoAnimation.tsx";

export type ChatContentProps = {
  onRetry: (index: number, question: UserMessage["content"]) => void;
  onStopStreaming: () => void;
};

export const ChatContent: React.FC<ChatContentProps> = ({
  onStopStreaming,
  onRetry,
}) => {
  const dispatch = useAppDispatch();
  const pauseReasonsWithPause = useAppSelector(selectThreadConfirmation);
  const messages = useAppSelector(selectMessages);
  const queuedMessages = useAppSelector(selectQueuedMessages);
  const isStreaming = useAppSelector(selectIsStreaming);
  const thread = useAppSelector(selectThread);

  const isConfig = thread?.mode === "CONFIGURE";
  const isWaiting = useAppSelector(selectIsWaiting);
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();
  const integrationMeta = useAppSelector(selectIntegration);
  const isWaitingForConfirmation = useAppSelector(selectThreadPause);

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
        projectPath: thread?.integration?.project,
        integrationName: thread?.integration?.name,
        integrationPath: thread?.integration?.path,
        wasOpenedThroughChat: true,
      }),
    );
  }, [
    onStopStreaming,
    dispatch,
    thread?.integration?.project,
    thread?.integration?.name,
    thread?.integration?.path,
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
        {messages.length === 0 && (
          <Container>
            <PlaceHolderText />
          </Container>
        )}
        {renderMessages(messages, onRetryWrapper, isWaiting)}
        {queuedMessages.length > 0 && (
          <Flex direction="column" gap="2" mt="2">
            {queuedMessages.map((queuedMsg, index) => (
              <QueuedMessage
                key={queuedMsg.id}
                queuedMessage={queuedMsg}
                position={index + 1}
              />
            ))}
          </Flex>
        )}
        <Container>
          <UncommittedChangesWarning />
        </Container>
        <Container pt="4" pb="8">
          {!isWaitingForConfirmation && (
            <LogoAnimation
              size="8"
              isStreaming={isStreaming}
              isWaiting={isWaiting}
            />
          )}
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
            {(isWaiting || isStreaming) && !pauseReasonsWithPause.pause && (
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

    // Find context_file messages that follow this assistant message (skipping tool messages)
    const contextFilesAfter: React.ReactNode[] = [];
    let skipCount = 0;
    let tempTail = tail;

    // Skip tool messages and collect context_file messages until we hit another message type
    while (tempTail.length > 0) {
      const nextMsg = tempTail[0];
      if (isToolMessage(nextMsg)) {
        // Skip tool messages (they're handled internally)
        skipCount++;
        tempTail = tempTail.slice(1);
      } else if (isChatContextFileMessage(nextMsg)) {
        // Collect context_file messages to render after assistant
        const ctxKey = "context-file-" + (index + 1 + skipCount);
        contextFilesAfter.push(<ContextFiles key={ctxKey} files={nextMsg.content} />);
        skipCount++;
        tempTail = tempTail.slice(1);
      } else {
        // Stop at any other message type (user, assistant, etc.)
        break;
      }
    }

    const nextMemo = [
      ...memo,
      <AssistantInput
        key={key}
        message={head.content}
        reasoningContent={head.reasoning_content}
        thinkingBlocks={head.thinking_blocks}
        toolCalls={head.tool_calls}
        serverExecutedTools={head.server_executed_tools}
        citations={head.citations}
      />,
      ...contextFilesAfter,
      <MessageUsageInfo
        key={`usage-${key}`}
        usage={head.usage}
        metering_coins_prompt={head.metering_coins_prompt}
        metering_coins_generated={head.metering_coins_generated}
        metering_coins_cache_creation={head.metering_coins_cache_creation}
        metering_coins_cache_read={head.metering_coins_cache_read}
      />,
    ];

    // Skip the tool and context_file messages we already processed
    const newTail = tail.slice(skipCount);
    return renderMessages(newTail, onRetry, waiting, nextMemo, index + 1 + skipCount);
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
