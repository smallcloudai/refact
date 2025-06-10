import React, { useCallback, useEffect, useMemo } from "react";
import { UserInput } from "../ChatContent/UserInput";
import { AssistantInput } from "../ChatContent/AssistantInput";
import {
  isAssistantMessage,
  isChatContextFileMessage,
  isChatMessage,
  isDiffMessage,
  isPlainTextMessage,
  isUserMessage,
} from "../../services/refact";
import { Container, Flex, IconButton, Text } from "@radix-ui/themes";
import { ArrowLeftIcon, ArrowRightIcon } from "@radix-ui/react-icons";
import { PlainText } from "../ChatContent/PlainText";
import { ContextFiles } from "../ChatContent/ContextFiles";
import { GroupedDiffs } from "../ChatContent/DiffContent";

import { FTMMessageNode as FTMessageNode } from "../../features/ThreadMessages/makeMessageTrie";
import { setThreadEnd } from "../../features/ThreadMessages";
import { useAppDispatch } from "../../hooks/useAppDispatch";

const ElementForNodeMessage: React.FC<{ message: FTMessageNode["value"] }> = ({
  message,
}) => {
  if (!isChatMessage(message)) return false;

  if (isUserMessage(message)) {
    return <UserInput>{message.ftm_content}</UserInput>;
  }

  if (isAssistantMessage(message)) {
    // find the tool result for the tool cal

    return (
      <AssistantInput toolCalls={message.tool_calls}>
        {message.ftm_content}
      </AssistantInput>
    );
  }

  if (isPlainTextMessage(message)) {
    return <PlainText>{message.ftm_content}</PlainText>;
  }

  if (isChatContextFileMessage(message)) {
    // TODO: why is this a linter error?
    return <ContextFiles files={message.ftm_content} />;
  }

  if (isDiffMessage(message)) {
    // TODO: do we still need to group diffs?
    return <GroupedDiffs diffs={[message]} />;
  }

  // add more case here from refact-agent/gui/src/components/ChatContent/ChatContent.tsx

  return false;
};

export type MessageNodeProps = { children: FTMessageNode };

export const MessageNode: React.FC<MessageNodeProps> = ({ children }) => {
  const dispatch = useAppDispatch();
  const [selectedNodeIndex, setSelectedNodeIndex] = React.useState<number>(0);
  // TODO: move this up?
  useEffect(() => {
    if (children.children.length === 0) {
      const action = setThreadEnd({
        number: children.value.ftm_num,
        alt: children.value.ftm_alt,
        prevAlt: children.value.ftm_prev_alt,
      });
      dispatch(action);
    }
  }, [
    children.children.length,
    children.value.ftm_alt,
    children.value.ftm_num,
    children.value.ftm_prev_alt,
    dispatch,
  ]);

  const goBack = useCallback(() => {
    setSelectedNodeIndex((prev) => {
      if (prev === 0) return prev;
      return prev - 1;
    });
  }, []);

  const goForward = useCallback(() => {
    setSelectedNodeIndex((prev) => {
      if (prev === children.children.length) return prev;
      return prev + 1;
    });
  }, [children.children.length]);

  const canBranch = useMemo(() => {
    if (children.children.length > 1) return true;
    if (children.value.ftm_role === "user") return true;
    return false;
  }, [children]);

  const nodeToRender = useMemo(() => {
    if (children.children.length === 0) return null;
    if (selectedNodeIndex > children.children.length) {
      return makeDummyNode(children.children[children.children.length - 1]);
    }
    return children.children[selectedNodeIndex];
  }, [children, selectedNodeIndex]);

  return (
    <>
      <ElementForNodeMessage message={children.value} />

      {canBranch && (
        <NodeSelectButtons
          onBackward={goBack}
          onForward={goForward}
          currentNode={selectedNodeIndex}
          totalNodes={children.children.length}
        />
      )}
      {nodeToRender && <MessageNode>{nodeToRender}</MessageNode>}
    </>
  );
};

const NodeSelectButtons: React.FC<{
  onForward: () => void;
  onBackward: () => void;
  currentNode: number;
  totalNodes: number;
}> = ({ onForward, onBackward, currentNode, totalNodes }) => {
  return (
    <Container my="2">
      <Flex gap="2" justify="start">
        <IconButton
          variant="ghost"
          size="1"
          disabled={currentNode === 0}
          radius="large"
          onClick={onBackward}
        >
          <ArrowLeftIcon />
        </IconButton>
        <Text size="1">
          {currentNode + 1} / {totalNodes}
        </Text>
        <IconButton
          variant="ghost"
          size="1"
          disabled={currentNode === totalNodes}
          onClick={onForward}
          radius="large"
        >
          <ArrowRightIcon />
        </IconButton>
      </Flex>
    </Container>
  );
};

function makeDummyNode(lastMessage?: FTMessageNode): FTMessageNode {
  // TODO handel the numbers better
  const num = lastMessage ? lastMessage.value.ftm_num - 1 : 0;
  return {
    value: {
      ftm_belongs_to_ft_id: lastMessage?.value.ftm_belongs_to_ft_id ?? "",
      ftm_alt: (lastMessage?.value.ftm_alt ?? 100) + 1,
      ftm_num: num,
      ftm_prev_alt: lastMessage?.value.ftm_prev_alt ?? 100,
      ftm_role: "", // TODO: maybe add a message.
      ftm_content: "",
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: Date.now(),
    },
    children: [],
  };
}
