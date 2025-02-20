import React from "react";
import {
  CMessageNode,
  isUserCMessageNode,
  UserCMessageNode,
} from "../../features/ChatDB/chatDbMessagesSlice";
import { UserInput } from "../ChatContent/UserInput";
import { AssistantInput } from "../ChatContent/AssistantInput";
import { ChatMessage } from "../../services/refact";
import { IconButton } from "@radix-ui/themes";
import { ArrowLeftIcon, ArrowRightIcon } from "@radix-ui/react-icons";

const ElementForNodeMessage: React.FC<{ message: ChatMessage }> = ({
  message,
}) => {
  if (message.role === "user") {
    return <UserInput>{message.content}</UserInput>;
  }

  if (message.role === "assistant") {
    return (
      <AssistantInput
        message={message.content}
        toolCalls={message.tool_calls}
      />
    );
  }

  return false;
};

export type MessageNodeProps = { children?: CMessageNode };
export const MessageNode: React.FC<MessageNodeProps> = ({ children }) => {
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
  const [selectedNodeIndex, setSelectedNodeIndex] = React.useState<number>(0);

  const selectedNode = children[selectedNodeIndex];
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
