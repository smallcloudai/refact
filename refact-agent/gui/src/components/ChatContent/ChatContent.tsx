import React, {
  forwardRef,
  useCallback,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  ChatMessages,
  isAssistantMessage,
  isChatContextFileMessage,
  isDiffMessage,
  isToolMessage,
  UserMessage,
} from "../../services/refact";
import { UserInput } from "./UserInput";
import { ScrollArea } from "../ScrollArea";
import { Spinner } from "../Spinner";
import { Flex, Container, Button, Box } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";
import { useAutoScroll } from "./useAutoScroll";
import { PlainText } from "./PlainText";
import { useAppDispatch } from "../../hooks";
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
import { ScrollToBottomButton } from "./ScrollToBottomButton";
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
  const scrollRef = useRef<HTMLDivElement>(null);
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

  const {
    handleScroll,
    handleWheel,
    handleScrollButtonClick,
    showFollowButton,
    scrollElementToTop,
    bottomSpace,
  } = useAutoScroll({
    scrollRef,
  });

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

  return (
    <ScrollArea
      ref={scrollRef}
      style={{ flexGrow: 1, height: "auto", position: "relative" }}
      scrollbars="vertical"
      onScroll={handleScroll}
      onWheel={handleWheel}
      type={isWaiting || isStreaming ? "auto" : "hover"}
      fullHeight
    >
      <Flex
        direction="column"
        className={styles.content}
        data-element="ChatContent"
        p="2"
        gap="1"
        // height="100%"
      >
        {messages.length === 0 && <PlaceHolderText />}
        {renderMessages(messages, onRetryWrapper, scrollElementToTop)}
        <UncommittedChangesWarning />
        {threadUsage && messages.length > 0 && <UsageCounter />}
        <Container py="4">
          <Spinner
            spinning={(isStreaming || isWaiting) && !isWaitingForConfirmation}
          />
        </Container>
        <ExtraSpace scrollRef={scrollRef} />
      </Flex>
      {showFollowButton && (
        <ScrollToBottomButton onClick={handleScrollButtonClick} />
      )}

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
    </ScrollArea>
  );
};

ChatContent.displayName = "ChatContent";

function renderMessages(
  messages: ChatMessages,
  onRetry: (index: number, question: UserMessage["content"]) => void,
  scrollElementToTop: (elem: HTMLElement) => void,
  memo: React.ReactNode[] = [],
  index = 0,
) {
  if (messages.length === 0) return memo;
  const [head, ...tail] = messages;
  if (head.role === "tool") {
    return renderMessages(tail, onRetry, scrollElementToTop, memo, index + 1);
  }

  if (head.role === "plain_text") {
    const key = "plain-text-" + index;
    const nextMemo = [...memo, <PlainText key={key}>{head.content}</PlainText>];
    return renderMessages(
      tail,
      onRetry,
      scrollElementToTop,
      nextMemo,
      index + 1,
    );
  }

  if (head.role === "assistant") {
    const key = "assistant-input-" + index;
    const isLast = !tail.some(isAssistantMessage);
    const nextMemo = [
      ...memo,
      <AssistantInput
        key={key}
        message={head.content}
        toolCalls={head.tool_calls}
        isLast={isLast}
      />,
    ];

    return renderMessages(
      tail,
      onRetry,
      scrollElementToTop,
      nextMemo,
      index + 1,
    );
  }

  if (head.role === "user") {
    const key = "user-input-" + index;
    // if last message scroll this message to the top of the bar
    const nextMemo = [
      ...memo,
      <UserInput
        onRetry={onRetry}
        key={key}
        messageIndex={index}
        forceScroll={tail.length === 0}
        scrollElementToTop={scrollElementToTop}
      >
        {head.content}
      </UserInput>,
    ];
    return renderMessages(
      tail,
      onRetry,
      scrollElementToTop,
      nextMemo,
      index + 1,
    );
  }

  if (isChatContextFileMessage(head)) {
    const key = "context-file-" + index;
    const nextMemo = [...memo, <ContextFiles key={key} files={head.content} />];
    return renderMessages(
      tail,
      onRetry,
      scrollElementToTop,
      nextMemo,
      index + 1,
    );
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
      scrollElementToTop,
      nextMemo,
      index + diffMessages.length,
    );
  }

  return renderMessages(tail, onRetry, scrollElementToTop, memo, index + 1);
}

const ExtraSpace: React.FC<{ scrollRef: React.RefObject<HTMLDivElement> }> = ({
  scrollRef,
}) => {
  const userMessages = scrollRef.current?.querySelectorAll(
    "[data-element='UserInput']",
  );
  const flexContainer = scrollRef.current?.querySelector(
    "[data-element='ChatContent']",
  );

  const [height, setHeight] = useState<number>(0);

  // top will be minus if off screen

  useLayoutEffect(() => {
    if (
      !scrollRef.current ||
      !userMessages ||
      !flexContainer ||
      userMessages.length === 0
    ) {
      return;
    }

    const lastUserMessage = userMessages[userMessages.length - 1];
    // const userMessageRect = lastUserMessage.getBoundingClientRect();
    // extends the height of the flex area
    if (flexContainer.clientHeight + height < scrollRef.current.clientHeight) {
      const h = scrollRef.current.clientHeight - flexContainer.clientHeight;
      const scrollTop = Math.max(lastUserMessage.scrollTop, 0);
      setHeight(h + scrollTop);
      return;
    }

    // flex is as big as it needs to be.
    // check that the top of the user message is at least client height from the bottom of th scroll
    const lastUserMessageRect = lastUserMessage.getBoundingClientRect();
    const scrollRect = scrollRef.current.getBoundingClientRect();

    // Calculate the distance from the top of the last user message to the bottom of the scroll area
    const distanceToBottom = scrollRect.bottom - lastUserMessageRect.top;

    // If the distance is less than the client height, add more space
    if (distanceToBottom + height < scrollRef.current.clientHeight) {
      const additionalHeight =
        scrollRef.current.clientHeight - distanceToBottom;
      // setHeight(height + additionalHeight);
      setHeight(additionalHeight);
      return;
    }
  }, [flexContainer, height, scrollRef, userMessages]);

  return <Box style={{ height: `${height}px` }} />;
};
