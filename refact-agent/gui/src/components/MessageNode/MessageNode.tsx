import React, { useCallback, useEffect, useMemo } from "react";
import { UserInput } from "../ChatContent/UserInput";
import { AssistantInput } from "../ChatContent/AssistantInput";
import {
  ChatContextFile,
  isAssistantMessage,
  isChatContextFileMessage,
  isChatMessage,
  isDiffMessage,
  isPlainTextMessage,
  isUserMessage,
} from "../../services/refact";
import { PlainText } from "../ChatContent/PlainText";
import { ContextFiles } from "../ChatContent/ContextFiles";
import { GroupedDiffs } from "../ChatContent/DiffContent";

import { FTMMessageNode as FTMessageNode } from "../../features/ThreadMessages/makeMessageTrie";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectMessageIsLastOfType,
  selectThreadMessageTopAltNumber,
  setThreadEnd,
} from "../../features/ThreadMessages";
import { useAppDispatch } from "../../hooks/useAppDispatch";
import { type NodeSelectButtonsProps } from "../ChatContent/UserInput";
import { useAppSelector } from "../../hooks";
import { parseOrElse } from "../../utils";
import { ScrollAreaWithAnchor } from "../ScrollArea";

const ElementForNodeMessage: React.FC<{
  message: FTMessageNode["value"];
  branch?: NodeSelectButtonsProps;
}> = ({ message, branch }) => {
  if (!isChatMessage(message)) return false;

  if (isUserMessage(message)) {
    return <UserInput branch={branch}>{message.ftm_content}</UserInput>;
  }

  if (isAssistantMessage(message)) {
    // find the tool result for the tool cal

    // TODO: why is this an error?, could be FTMessageNode ?
    return (
      <AssistantInput toolCalls={message.ftm_tool_calls}>
        {message.ftm_content}
      </AssistantInput>
    );
  }

  if (isPlainTextMessage(message)) {
    return <PlainText>{message.ftm_content}</PlainText>;
  }

  if (isChatContextFileMessage(message)) {
    const files = parseOrElse<ChatContextFile[]>(message.ftm_content, []);
    return <ContextFiles files={files} />;
  }

  if (isDiffMessage(message)) {
    return <GroupedDiffs diffs={[message]} />;
  }

  // add more case here from refact-agent/gui/src/components/ChatContent/ChatContent.tsx

  return false;
};

export type MessageNodeProps = {
  children: FTMessageNode;
  branch?: NodeSelectButtonsProps;
};

export const MessageNode: React.FC<MessageNodeProps> = ({
  children,
  branch,
}) => {
  const dispatch = useAppDispatch();

  const isLastOfRole = useAppSelector((state) =>
    selectMessageIsLastOfType(state, children.value),
  );
  const isWaiting = useAppSelector(selectIsWaiting);
  const isStreaming = useAppSelector(selectIsStreaming);

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

  return (
    <>
      {/**TODO: this could be put at the end of the assistant message */}
      {/**TODO: has to resize on some changes */}
      {!isWaiting &&
        !isStreaming &&
        children.value.ftm_role === "user" &&
        isLastOfRole && (
          <ScrollAreaWithAnchor.ScrollAnchor behavior="smooth" block="start" />
        )}
      <ElementForNodeMessage branch={branch} message={children.value} />
      <MessageNodeChildren>{children.children}</MessageNodeChildren>
    </>
  );
};

// type NodeSelectButtonsProps = {
//   onForward: () => void;
//   onBackward: () => void;
//   currentNode: number;
//   totalNodes: number;
// };

// const NodeSelectButtons: React.FC<NodeSelectButtonsProps> = ({
//   onForward,
//   onBackward,
//   currentNode,
//   totalNodes,
// }) => {
//   return (
//     <Container my="2">
//       <Flex gap="2" justify="start">
//         <IconButton
//           variant="ghost"
//           size="1"
//           disabled={currentNode === 0}
//           radius="large"
//           onClick={onBackward}
//         >
//           <ArrowLeftIcon />
//         </IconButton>
//         <Text size="1">
//           {currentNode + 1} / {totalNodes}
//         </Text>
//         <IconButton
//           variant="ghost"
//           size="1"
//           disabled={currentNode === totalNodes}
//           onClick={onForward}
//           radius="large"
//         >
//           <ArrowRightIcon />
//         </IconButton>
//       </Flex>
//     </Container>
//   );
// };

function makeDummyNode(
  ft_id?: string,
  lastMessageNumber?: number,
  altNumber?: number,
  prevAlt?: number,
): FTMessageNode {
  // TODO handel the numbers better
  const num = lastMessageNumber ? lastMessageNumber - 1 : 0;
  return {
    value: {
      ftm_belongs_to_ft_id: ft_id ?? "",
      ftm_alt: (altNumber ?? 100) + 1,
      ftm_num: num,
      ftm_prev_alt: prevAlt ?? 100,
      ftm_role: "user", // TODO: maybe add a message.
      ftm_content: "",
      ftm_tool_calls: null,
      ftm_call_id: "",
      ftm_usage: null,
      ftm_created_ts: Date.now(),
    },
    children: [],
  };
}

function useThreadBranching(children: FTMessageNode[]) {
  const [selectedNodeIndex, setSelectedNodeIndex] = React.useState<number>(0);
  const currentMaxAlt = useAppSelector(selectThreadMessageTopAltNumber);

  const onBackward = useCallback(() => {
    setSelectedNodeIndex((prev) => {
      if (prev === 0) return prev;
      return prev - 1;
    });
  }, []);

  const onForward = useCallback(() => {
    setSelectedNodeIndex((prev) => {
      if (prev === children.length) return prev;
      return prev + 1;
    });
  }, [children.length]);

  const canBranch = useMemo(() => {
    if (children.length > 1) return true;
    if (selectedNodeIndex >= children.length && selectedNodeIndex > 0) {
      return true;
    }

    if (
      children[selectedNodeIndex] &&
      children[selectedNodeIndex].value.ftm_role === "user"
    ) {
      return true;
    }
    return false;
  }, [children, selectedNodeIndex]);

  const nodeToRender = useMemo(() => {
    if (children[selectedNodeIndex]) return children[selectedNodeIndex];
    const lastChild =
      children.length === 0 ? null : children[children.length - 1];

    return makeDummyNode(
      lastChild?.value.ftm_belongs_to_ft_id,
      lastChild?.value.ftm_num,
      currentMaxAlt ?? 100,
      lastChild?.value.ftm_prev_alt,
    );
  }, [children, currentMaxAlt, selectedNodeIndex]);

  return {
    onForward,
    onBackward,
    currentNode: selectedNodeIndex,
    totalNodes: children.length,
    nodeToRender,
    canBranch,
    selectedNodeIndex,
  };
}

const MessageNodeChildren: React.FC<{ children: FTMessageNode[] }> = ({
  children,
}) => {
  const { nodeToRender, canBranch, ...branch } = useThreadBranching(children);

  if (children.length === 0) return null;

  if (!canBranch) {
    return <MessageNode>{nodeToRender}</MessageNode>;
  }

  return (
    <>
      <MessageNode branch={branch}>{nodeToRender}</MessageNode>
    </>
  );
};
