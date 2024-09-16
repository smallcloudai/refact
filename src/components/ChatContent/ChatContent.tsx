import React, { useCallback, useEffect } from "react";
import {
  ChatMessage,
  ChatMessages,
  DiffMessage,
  isChatContextFileMessage,
  isDiffMessage,
} from "../../services/refact";
import { UserInput } from "./UserInput";
import { ScrollArea } from "../ScrollArea";
import { Spinner } from "../Spinner";
import { Flex, Text, Container, Link } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";
import { ContextFiles } from "./ContextFiles";
import { AssistantInput } from "./AssistantInput";
import { MemoryContent } from "./MemoryContent";
import { useAutoScroll } from "./useAutoScroll";
import { DiffContent } from "./DiffContent";
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

export const ChatContent = React.forwardRef<HTMLDivElement>((_props, ref) => {
  const messages = useAppSelector(selectMessages);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);

  const { innerRef, handleScroll } = useAutoScroll({
    ref,
    messages,
    isStreaming,
  });

  return (
    <ScrollArea
      style={{ flexGrow: 1, height: "auto" }}
      scrollbars="vertical"
      onScroll={handleScroll}
    >
      <Flex direction="column" className={styles.content} p="2" gap="1">
        {messages.length === 0 && <PlaceHolderText />}
        {renderMessages(messages)}
        {isWaiting && (
          <Container py="4">
            <Spinner />
          </Container>
        )}
        <div ref={innerRef} />
      </Flex>
    </ScrollArea>
  );
});

ChatContent.displayName = "ChatContent";

function renderMessages(
  messages: ChatMessages,
  memo: React.ReactNode[] = [],
  index = 0,
) {
  if (messages.length === 0) return memo;
  const [head, ...tail] = messages;
  if (head.role === "tool") {
    return renderMessages(tail, memo, index + 1);
  }

  if (head.role === "context_memory") {
    const key = "context-memory-" + index;
    const nextMemo = [
      ...memo,
      <MemoryContent key={key} items={head.content} />,
    ];
    return renderMessages(tail, nextMemo, index + 1);
  }

  if (head.role === "plain_text") {
    const key = "plain-text-" + index;
    const nextMemo = [...memo, <PlainText key={key}>{head.content}</PlainText>];
    return renderMessages(tail, nextMemo, index + 1);
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

    return renderMessages(tail, nextMemo, index + 1);
  }

  if (head.role === "user") {
    const key = "user-input-" + index;
    const nextMemo = [
      ...memo,
      <UserInput key={key} messageIndex={index}>
        {head.content}
      </UserInput>,
    ];
    return renderMessages(tail, nextMemo, index + 1);
  }

  if (isChatContextFileMessage(head)) {
    const key = "context-file-" + head.content.length;
    const nextMemo = [...memo, <ContextFiles key={key} files={head.content} />];
    return renderMessages(tail, nextMemo, index + 1);
  }

  if (isDiffMessage(head)) {
    const restInTail = takeWhile<ChatMessage, DiffMessage>(tail, isDiffMessage);
    const nextTail = tail.slice(restInTail.length);
    const diffs = [head, ...restInTail];
    console.log({ restInTail, diffs });
    const key = "diffs-" + index;

    const nextMemo = [...memo, <GroupedDiffs key={key} diffs={diffs} />];

    return renderMessages(nextTail, nextMemo, index + diffs.length);
  }

  return renderMessages(tail, memo, index + 1);
}

type GroupedDiffsProps = {
  diffs: DiffMessage[];
};
const GroupedDiffs: React.FC<GroupedDiffsProps> = ({ diffs }) => {
  return (
    <Flex direction="column" gap="2">
      {diffs.map((diff) => (
        <DiffContent
          key={diff.tool_call_id}
          chunks={diff.content}
          toolCallId={diff.tool_call_id}
        />
      ))}
      <div>TODO: new apply all button</div>
    </Flex>
  );
};
