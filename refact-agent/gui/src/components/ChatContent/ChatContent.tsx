import React, { useCallback, useMemo } from "react";

import { ScrollArea, ScrollAreaWithAnchor } from "../ScrollArea";
import { Flex, Container, Button, Box } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";

import { useAppDispatch, useDiffFileReload } from "../../hooks";
import { useAppSelector } from "../../hooks";
import {
  selectIntegration,
  selectThread,
} from "../../features/Chat/Thread/selectors";
import {
  selectIsStreaming,
  selectIsThreadRunning,
  selectIsWaiting,
  selectThreadId,
} from "../../features/ThreadMessages";

import { popBackTo } from "../../features/Pages/pagesSlice";
import { ChatLinks, UncommittedChangesWarning } from "../ChatLinks";
import { PlaceHolderText } from "./PlaceHolderText";
import { UsageCounter } from "../UsageCounter";
import {
  getConfirmationPauseStatus,
  getPauseReasonsWithPauseStatus,
} from "../../features/ToolConfirmation/confirmationSlice";
import { useUsageCounter } from "../UsageCounter/useUsageCounter.ts";
import { LogoAnimation } from "../LogoAnimation/LogoAnimation.tsx";
import { selectThreadMessageTrie } from "../../features/ThreadMessages";
import { MessageNode } from "../MessageNode/MessageNode.tsx";
import { isEmptyNode } from "../../features/ThreadMessages/makeMessageTrie.ts";
import { pauseThreadThunk } from "../../services/graphql/graphqlThunks.ts";

const usePauseThread = () => {
  const dispatch = useAppDispatch();
  const isThreadRunning = useAppSelector(selectIsThreadRunning);
  const threadId = useAppSelector(selectThreadId);
  const pauseReasonsWithPause = useAppSelector(getPauseReasonsWithPauseStatus);
  // TODO: hide during tool confimation pause
  const shouldShowStopButton = useMemo(() => {
    if (!threadId) return false;
    if (pauseReasonsWithPause.pause) return false;
    return isThreadRunning;
  }, [threadId, pauseReasonsWithPause.pause, isThreadRunning]);

  const handlePause = useCallback(() => {
    if (!threadId) return;
    void dispatch(pauseThreadThunk({ id: threadId }));
  }, [dispatch, threadId]);

  return { shouldShowStopButton, handlePause };
};

export const ChatContent: React.FC = () => {
  const dispatch = useAppDispatch();
  // TODO: stays when creating a new chat :/
  const threadMessageTrie = useAppSelector(selectThreadMessageTrie);
  const isStreaming = useAppSelector(selectIsStreaming);
  const thread = useAppSelector(selectThread);
  const { shouldShow } = useUsageCounter();
  const isConfig = thread.mode === "CONFIGURE";
  const isWaiting = useAppSelector(selectIsWaiting);

  const integrationMeta = useAppSelector(selectIntegration);
  const isWaitingForConfirmation = useAppSelector(getConfirmationPauseStatus);

  const { shouldShowStopButton, handlePause } = usePauseThread();

  const handleReturnToConfigurationClick = useCallback(() => {
    // console.log(`[DEBUG]: going back to configuration page`);
    // TBD: should it be allowed to run in the background?
    // onStopStreaming();
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
    // onStopStreaming,
    dispatch,
    thread.integration?.project,
    thread.integration?.name,
    thread.integration?.path,
  ]);

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
        {isEmptyNode(threadMessageTrie) ? (
          <Container>
            <PlaceHolderText />
          </Container>
        ) : (
          <MessageNode>{threadMessageTrie}</MessageNode>
        )}
        {/* {renderMessages(messages, onRetryWrapper, isWaiting)} */}
        <Container>
          <UncommittedChangesWarning />
        </Container>
        {shouldShow && <UsageCounter />}
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
            {shouldShowStopButton && (
              <Button
                // ml="auto"
                color="red"
                title="Pause thread"
                onClick={handlePause}
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

// function renderMessages(
//   messages: ChatMessages,
//   onRetry: (index: number, question: UserMessage["ftm_content"]) => void,
//   waiting: boolean,
//   memo: React.ReactNode[] = [],
//   index = 0,
// ) {
//   if (messages.length === 0) return memo;
//   const [head, ...tail] = messages;
//   if (head.role === "tool") {
//     return renderMessages(tail, onRetry, waiting, memo, index + 1);
//   }

//   if (head.role === "plain_text") {
//     const key = "plain-text-" + index;
//     const nextMemo = [
//       ...memo,
//       <PlainText key={key}>{head.ftm_content}</PlainText>,
//     ];
//     return renderMessages(tail, onRetry, waiting, nextMemo, index + 1);
//   }

//   if (head.role === "assistant") {
//     const key = "assistant-input-" + index;
//     const isLast = !tail.some(isAssistantMessage);
//     const nextMemo = [
//       ...memo,
//       <AssistantInput
//         key={key}
//         message={head.ftm_content}
//         reasoningContent={head.reasoning_content}
//         toolCalls={head.tool_calls}
//         isLast={isLast}
//       />,
//     ];

//     return renderMessages(tail, onRetry, waiting, nextMemo, index + 1);
//   }

//   if (head.role === "user") {
//     const key = "user-input-" + index;
//     const isLastUserMessage = !tail.some(isUserMessage);
//     const nextMemo = [
//       ...memo,
//       isLastUserMessage && (
//         <ScrollAreaWithAnchor.ScrollAnchor
//           key={`${key}-anchor`}
//           behavior="smooth"
//           block="start"
//           // my="-2"
//         />
//       ),
//       <UserInput onRetry={onRetry} key={key} messageIndex={index}>
//         {head.ftm_content}
//       </UserInput>,
//     ];
//     return renderMessages(tail, onRetry, waiting, nextMemo, index + 1);
//   }

//   if (isChatContextFileMessage(head)) {
//     const key = "context-file-" + index;
//     const nextMemo = [
//       ...memo,
//       <ContextFiles key={key} files={head.ftm_content} />,
//     ];
//     return renderMessages(tail, onRetry, waiting, nextMemo, index + 1);
//   }

//   if (isDiffMessage(head)) {
//     const restInTail = takeWhile(tail, (message) => {
//       return isDiffMessage(message) || isToolMessage(message);
//     });

//     const nextTail = tail.slice(restInTail.length);
//     const diffMessages = [head, ...restInTail.filter(isDiffMessage)];
//     const key = "diffs-" + index;

//     const nextMemo = [...memo, <GroupedDiffs key={key} diffs={diffMessages} />];

//     return renderMessages(
//       nextTail,
//       onRetry,
//       waiting,
//       nextMemo,
//       index + diffMessages.length,
//     );
//   }

//   return renderMessages(tail, onRetry, waiting, memo, index + 1);
// }
