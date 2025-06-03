import { partition } from "../../utils";
import { MessagesSubscriptionSubscription } from "../../../generated/documents";

export type Message = NonNullable<
  MessagesSubscriptionSubscription["comprehensive_thread_subs"]["news_payload_thread_message"]
>;

type EmptyNode = {
  value: null;
  children: null;
};
export type MessageNode =
  | {
      value: Message;
      children: MessageNode[];
    }
  | EmptyNode;

// const isRoot = (message: Message): boolean => {
//   return message.ftm_prev_alt === -1;
// };

export function sortMessageList(messages: Message[]): Message[] {
  return messages.slice(0).sort((a, b) => {
    if (a.ftm_num === b.ftm_num) {
      return a.ftm_alt - b.ftm_alt;
    }
    return a.ftm_num - b.ftm_num;
  });
}

export const makeMessageTrie = (messages: Message[]): MessageNode => {
  if (messages.length === 0) return { value: null, children: null };
  const sortedMessages = sortMessageList(messages);

  // const [nodes, roots] = partition(sortedMessages, isRoot);
  const [root, ...nodes] = sortedMessages;
  // if (roots.length === 0) return null;
  // TODO: handle multiple roots;
  // const root = roots[0];
  const children = getChildren(root, nodes);
  return {
    value: root,
    children,
  };
};

function getChildren(parent: Message, messages: Message[]): MessageNode[] {
  if (messages.length === 0) return [];
  const rowNumber = parent.ftm_num + 1;
  const [other, siblings] = partition(messages, (m) => {
    return m.ftm_num === rowNumber && m.ftm_prev_alt === parent.ftm_alt;
  });

  return siblings.map((s) => {
    return { value: s, children: getChildren(s, other) };
  });
}
