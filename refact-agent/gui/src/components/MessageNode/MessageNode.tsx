import React, { useCallback, useEffect, useMemo } from "react";
// import {
//   chatDbMessageSliceActions,
//   CMessageNode,
//   isUserCMessageNode,
// } from "../../features/ChatDB/chatDbMessagesSlice";
import { UserInput } from "../ChatContent/UserInput";
import { AssistantInput } from "../ChatContent/AssistantInput";
import {
  // ChatMessage,
  // ChatMessage,
  isAssistantMessage,
  isChatContextFileMessage,
  isChatMessage,
  isDiffMessage,
  isPlainTextMessage,
  isUserMessage,
} from "../../services/refact";
import { Box, Flex, IconButton } from "@radix-ui/themes";
import { ArrowLeftIcon, ArrowRightIcon } from "@radix-ui/react-icons";
import { PlainText } from "../ChatContent/PlainText";
import { ContextFiles } from "../ChatContent/ContextFiles";
import { GroupedDiffs } from "../ChatContent/DiffContent";

import { MessageNode as FTMessageNode } from "../../features/ThreadMessages/makeMessageTrie";

const ElementForNodeMessage: React.FC<{ message: FTMessageNode["value"] }> = ({
  message,
}) => {
  if (!isChatMessage(message)) return false;

  if (isUserMessage(message)) {
    return <UserInput>{message.ftm_content}</UserInput>;
  }

  if (isAssistantMessage(message)) {
    // find the tool result for the tool call
    return (
      <AssistantInput
        message={message.ftm_content}
        toolCalls={message.tool_calls}
      />
    );
  }

  if (isPlainTextMessage(message)) {
    return <PlainText>{message.ftm_content}</PlainText>;
  }

  if (isChatContextFileMessage(message)) {
    return <ContextFiles files={message.ftm_content} />;
  }

  if (isDiffMessage(message)) {
    // TODO: do we still need to group diffs?
    return <GroupedDiffs diffs={[message]} />;
  }

  // add more case here from refact-agent/gui/src/components/ChatContent/ChatContent.tsx

  return false;
};

export type MessageNodeProps = { children?: FTMessageNode | null };

// TODO: update tracking the end point
export const MessageNode: React.FC<MessageNodeProps> = ({ children }) => {
  // const dispatch = useAppDispatch();

  // useEffect(() => {
  //   if (children?.children.length === 0) {
  //     const action = chatDbMessageSliceActions.setEnd({
  //       number: children.message.cmessage_num,
  //       alt: children.message.cmessage_alt,
  //     });
  //     dispatch(action);
  //   }
  // }, [
  //   children?.children.length,
  //   children?.message.cmessage_num,
  //   children?.message.cmessage_alt,
  //   dispatch,
  // ]);

  if (!children) return null;
  return (
    <>
      <ElementForNodeMessage message={children.value} />
      {children.children && (
        <MessageNodeChildren>{children.children}</MessageNodeChildren>
      )}
    </>
  );
};

// function makeDummyNode(lastMessage?: FTMessageNode): FTMessageNode {
//   return {
//     message: {
//       cmessage_usage_model: lastMessage?.message.cmessage_usage_model ?? "",
//       cmessage_usage_prompt: lastMessage?.message.cmessage_usage_prompt ?? 0,
//       cmessage_usage_completion:
//         lastMessage?.message.cmessage_usage_completion ?? 0,
//       cmessage_belongs_to_cthread_id:
//         lastMessage?.message.cmessage_belongs_to_cthread_id ?? "",
//       cmessage_num: lastMessage?.message.cmessage_num ?? 0,

//       cmessage_alt: (lastMessage?.message.cmessage_alt ?? 0) + 1,
//       cmessage_prev_alt: lastMessage?.message.cmessage_alt ?? 0,
//       cmessage_json: {
//         role: "user",
//         content: "dummy text about making a new message",
//       }, // TODO: use a different type of message
//     },
//     children: [],
//   };
// }

const MessageNodeChildren: React.FC<{ children: FTMessageNode[] }> = ({
  children,
}) => {
  const [selectedNodeIndex, setSelectedNodeIndex] = React.useState<number>(0);

  const goBack = useCallback(() => {
    setSelectedNodeIndex((prev) => {
      if (prev === 0) return prev;
      return prev - 1;
    });
  }, []);

  const goForward = useCallback(() => {
    setSelectedNodeIndex((prev) => {
      if (prev === children.length) return prev;
      return prev + 1;
    });
  }, [children.length]);

  const canBranch = useMemo(() => {
    if (children.length > 1) return true;
    // if (selectedNodeIndex >= children.length && selectedNodeIndex > 0) {
    //   return true;
    // }
    // if (
    //   children[selectedNodeIndex] &&
    //   isUserCMessageNode(children[selectedNodeIndex])
    // ) {
    //   return true;
    // }
    // return false;
  }, [children]);

  const nodeToRender = useMemo(() => {
    return children[0];
    // return (
    //   children[selectedNodeIndex] ??
    //   makeDummyNode(children[children.length - 1])
    // );
  }, [children]);

  if (!canBranch) {
    return <MessageNode>{children[selectedNodeIndex]}</MessageNode>;
  }

  return (
    <Box>
      <Flex gap="4" justify="end">
        <IconButton
          variant="outline"
          size="1"
          disabled={selectedNodeIndex === 0}
          onClick={goBack}
        >
          <ArrowLeftIcon />
        </IconButton>
        <IconButton
          variant="outline"
          size="1"
          disabled={selectedNodeIndex === children.length}
          onClick={goForward}
        >
          <ArrowRightIcon />
        </IconButton>
      </Flex>
      <MessageNode>{nodeToRender}</MessageNode>
    </Box>
  );
};
