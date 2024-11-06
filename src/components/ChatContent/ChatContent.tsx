import React, { useCallback, useEffect } from "react";
import {
  ChatMessages,
  isChatContextFileMessage,
  isDiffMessage,
  isToolMessage,
  UserMessage,
} from "../../services/refact";
import { UserInput } from "./UserInput";
import { ScrollArea } from "../ScrollArea";
import { Spinner } from "../Spinner";
import { Flex, Text, Container, Link } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";
import { useAutoScroll } from "./useAutoScroll";
import { PlainText } from "./PlainText";
import { useConfig, useEventsBusForIDE } from "../../hooks";
import { useAppSelector, useAppDispatch } from "../../hooks";
import { RootState } from "../../app/store";
import { next } from "../../features/TipOfTheDay";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
} from "../../features/Chat/Thread/selectors";
import { takeWhile } from "../../utils";
import { GroupedDiffs } from "./DiffContent";
import { ScrollToBottomButton } from "./ScrollToBottomButton";

export const TipOfTheDay: React.FC = () => {
  const dispatch = useAppDispatch();
  const config = useConfig();
  const state = useAppSelector((state: RootState) => state.tipOfTheDay);

  // TODO: find out what this is about.
  useEffect(() => {
    dispatch(next(config));
  }, [dispatch, config]);

  return (
    <Text>
      💡 <b>Tip of the day</b>: {state.tip}
    </Text>
  );
};

// TODO: turn this into a component
const PlaceHolderText: React.FC = () => {
  const config = useConfig();
  const hasVecDB = config.features?.vecdb ?? false;
  const hasAst = config.features?.ast ?? false;
  const { openSettings } = useEventsBusForIDE();

  const handleOpenSettings = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault();
      openSettings();
    },
    [openSettings],
  );

  if (config.host === "web") {
    <Flex direction="column" gap="4">
      <Text>Welcome to Refact chat!</Text>;
      <TipOfTheDay />
    </Flex>;
  }

  if (!hasVecDB && !hasAst) {
    return (
      <Flex direction="column" gap="4">
        <Text>Welcome to Refact chat!</Text>
        <Text>
          💡 You can turn on VecDB and AST in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  } else if (!hasVecDB) {
    return (
      <Flex direction="column" gap="4">
        <Text>Welcome to Refact chat!</Text>
        <Text>
          💡 You can turn on VecDB in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  } else if (!hasAst) {
    return (
      <Flex direction="column" gap="4">
        <Text>Welcome to Refact chat!</Text>
        <Text>
          💡 You can turn on AST in{" "}
          <Link onClick={handleOpenSettings}>settings</Link>.
        </Text>
        <TipOfTheDay />
      </Flex>
    );
  }

  return (
    <Flex direction="column" gap="4">
      <Text>Welcome to Refact chat.</Text>
      <TipOfTheDay />
    </Flex>
  );
};

export type ChatContentProps = {
  onRetry: (index: number, question: UserMessage["content"]) => void;
};

export const ChatContent = React.forwardRef<HTMLDivElement, ChatContentProps>(
  (props, ref) => {
    const messages = useAppSelector(selectMessages);
    const isStreaming = useAppSelector(selectIsStreaming);
    const isWaiting = useAppSelector(selectIsWaiting);

    const {
      innerRef,
      handleScroll,
      handleWheel,
      handleScrollButtonClick,
      isScrolledTillBottom,
    } = useAutoScroll({
      ref,
      messages,
      isStreaming,
    });

    const onRetryWrapper = (
      index: number,
      question: UserMessage["content"],
    ) => {
      props.onRetry(index, question);
      handleScrollButtonClick();
    };

    return (
      <ScrollArea
        style={{ flexGrow: 1, height: "auto", position: "relative" }}
        scrollbars="vertical"
        onScroll={handleScroll}
        onWheel={handleWheel}
      >
        <Flex direction="column" className={styles.content} p="2" gap="1">
          {messages.length === 0 && <PlaceHolderText />}
          {renderMessages(messages, onRetryWrapper)}
          {isWaiting && (
            <Container py="4">
              <Spinner />
            </Container>
          )}
          <div ref={innerRef} />
        </Flex>
        {!isScrolledTillBottom && (
          <ScrollToBottomButton onClick={handleScrollButtonClick} />
        )}
      </ScrollArea>
    );
  },
);

ChatContent.displayName = "ChatContent";

function renderMessages(
  messages: ChatMessages,
  onRetry: (index: number, question: UserMessage["content"]) => void,
  memo: React.ReactNode[] = [],
  index = 0,
) {
  if (messages.length === 0) return memo;
  const [head, ...tail] = messages;
  if (head.role === "tool") {
    return renderMessages(tail, onRetry, memo, index + 1);
  }

  if (head.role === "plain_text") {
    const key = "plain-text-" + index;
    const nextMemo = [...memo, <PlainText key={key}>{head.content}</PlainText>];
    return renderMessages(tail, onRetry, nextMemo, index + 1);
  }

  if (head.role === "assistant") {
    const key = "assistant-input-" + index;
    const nextMemo = [
      ...memo,
      <AssistantInput
        key={key}
        message={head.content}
        toolCalls={head.tool_calls}
      />,
    ];

    return renderMessages(tail, onRetry, nextMemo, index + 1);
  }

  if (head.role === "user") {
    const key = "user-input-" + index;

    const nextMemo = [
      ...memo,
      <UserInput onRetry={onRetry} key={key} messageIndex={index}>
        {head.content}
      </UserInput>,
    ];
    return renderMessages(tail, onRetry, nextMemo, index + 1);
  }

  if (isChatContextFileMessage(head)) {
    const key = "context-file-" + index;
    const nextMemo = [...memo, <ContextFiles key={key} files={head.content} />];
    return renderMessages(tail, onRetry, nextMemo, index + 1);
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
      nextMemo,
      index + diffMessages.length,
    );
  }

  return renderMessages(tail, onRetry, memo, index + 1);
}
