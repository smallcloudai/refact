import { CMessage } from "../../services/refact";
import { CMessageNode } from "./chatDbMessagesSlice";
import { partition } from "../../utils";

const isRoot = (message: CMessage): boolean => {
  return message.cmessage_prev_alt === -1;
};

export function sortMessageList(messages: CMessage[]): CMessage[] {
  return messages.slice(0).sort((a, b) => {
    if (a.cmessage_num === b.cmessage_num) {
      return a.cmessage_alt - b.cmessage_alt;
    }
    return a.cmessage_num - b.cmessage_num;
  });
}

export const makeMessageTree = (messages: CMessage[]): CMessageNode | null => {
  const sortedMessages = sortMessageList(messages);

  const [nodes, roots] = partition(sortedMessages, isRoot);
  if (roots.length === 0) return null;
  // TODO: handle multiple roots;
  const root = roots[0];
  const children = getChildren(root, nodes);
  return {
    message: root,
    children,
  };
};

function getChildren(parent: CMessage, messages: CMessage[]): CMessageNode[] {
  if (messages.length === 0) return [];
  const rowNumber = parent.cmessage_num + 1;
  const [other, siblings] = partition(messages, (m) => {
    return (
      m.cmessage_num === rowNumber &&
      m.cmessage_prev_alt === parent.cmessage_alt
    );
  });

  return siblings.map((s) => {
    return { message: s, children: getChildren(s, other) };
  });
}
