import React, { useEffect, useMemo } from "react";
import {
  chatDbMessageSliceActions,
  CMessageNode,
  isUserCMessageNode,
  UserCMessageNode,
} from "../../features/ChatDB/chatDbMessagesSlice";
import { UserInput } from "../ChatContent/UserInput";
import { AssistantInput } from "../ChatContent/AssistantInput";
import {
  ChatMessage,
  isAssistantMessage,
  isChatContextFileMessage,
  isDiffMessage,
  isPlainTextMessage,
  isUserMessage,
} from "../../services/refact";
import { IconButton } from "@radix-ui/themes";
import { ArrowLeftIcon, ArrowRightIcon } from "@radix-ui/react-icons";
import { PlainText } from "../ChatContent/PlainText";
import { ContextFiles } from "../ChatContent/ContextFiles";
import { GroupedDiffs } from "../ChatContent/DiffContent";
import { useAppDispatch } from "../../hooks";

const ElementForNodeMessage: React.FC<{ message: ChatMessage }> = ({
  message,
}) => {
  if (isUserMessage(message)) {
    return <UserInput>{message.content}</UserInput>;
  }

  if (isAssistantMessage(message)) {
    // find the tool result for the tool call
    return (
      <AssistantInput
        message={message.content}
        toolCalls={message.tool_calls}
      />
    );
  }

  if (isPlainTextMessage(message)) {
    return <PlainText>{message.content}</PlainText>;
  }

  if (isChatContextFileMessage(message)) {
    return <ContextFiles files={message.content} />;
  }

  if (isDiffMessage(message)) {
    // TODO: do we still need to group diffs?
    return <GroupedDiffs diffs={[message]} />;
  }

  // add more case here from refact-agent/gui/src/components/ChatContent/ChatContent.tsx

  return false;
};

export type MessageNodeProps = { children?: CMessageNode | null };

// TODO: update tracking the end point
export const MessageNode: React.FC<MessageNodeProps> = ({ children }) => {
  const dispatch = useAppDispatch();

  useEffect(() => {
    if (children?.children.length === 0) {
      const action = chatDbMessageSliceActions.setEnd({
        number: children.message.cmessage_num,
        alt: children.message.cmessage_alt,
      });
      dispatch(action);
    }
  }, [
    children?.children.length,
    children?.message.cmessage_num,
    children?.message.cmessage_alt,
    dispatch,
  ]);

  if (!children) return null;
  return (
    <>
      <ElementForNodeMessage message={children.message.cmessage_json} />
      <MessageNodeChildren>{children.children}</MessageNodeChildren>
    </>
  );
};

const MessageNodeChildren: React.FC<{ children: CMessageNode[] }> = ({
  children,
}) => {
  const userMessages: UserCMessageNode[] = children.filter(isUserCMessageNode);

  if (userMessages.length === 0) {
    return children.map((node, index) => {
      const key = `${node.message.cmessage_belongs_to_cthread_id}_${node.message.cmessage_num}_${node.message.cmessage_alt}_${index}`;
      return <MessageNode key={key}>{node}</MessageNode>;
    });
  } else {
    return <UserMessageNode>{userMessages}</UserMessageNode>;
  }
};

const UserMessageNode: React.FC<{ children: UserCMessageNode[] }> = ({
  children,
}) => {
  // info about the node may need to be shared with the user input
  const dispatch = useAppDispatch();
  const [selectedNodeIndex, setSelectedNodeIndex] = React.useState<number>(0);

  const selectedNode = children[selectedNodeIndex];

  useEffect(() => {
    if (selectedNode.children.length === 0) {
      const action = chatDbMessageSliceActions.setEnd({
        number: selectedNode.message.cmessage_num,
        alt: selectedNode.message.cmessage_alt,
      });
      dispatch(action);
    }
  }, [
    selectedNode.children.length,
    selectedNode.message.cmessage_num,
    selectedNode.message.cmessage_alt,
    dispatch,
  ]);
  return (
    <>
      <IconButton
        variant="outline"
        size="1"
        disabled={selectedNodeIndex === 0}
        onClick={() =>
          setSelectedNodeIndex((prev) => {
            if (prev === 0) return prev;
            return prev - 1;
          })
        }
      >
        <ArrowLeftIcon />
      </IconButton>
      <IconButton
        variant="outline"
        size="1"
        disabled={selectedNodeIndex === children.length - 1}
        onClick={() => {
          setSelectedNodeIndex((prev) => {
            if (prev === children.length - 1) return prev;
            return prev + 1;
          });
        }}
      >
        <ArrowRightIcon />
      </IconButton>
      <UserInput>{selectedNode.message.cmessage_json.content}</UserInput>
      <MessageNodeChildren>{selectedNode.children}</MessageNodeChildren>
    </>
  );
};
