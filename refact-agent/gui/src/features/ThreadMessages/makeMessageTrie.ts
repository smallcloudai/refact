import { partition } from "../../utils";
import { MessagesSubscriptionSubscription } from "../../../generated/documents";

export type FTMMessage = NonNullable<
  MessagesSubscriptionSubscription["comprehensive_thread_subs"]["news_payload_thread_message"]
>;

interface Node<T> {
  value: T;
  children: Node<T>[];
}

type EmptyNode = Node<null>;

export type FTMMessageNode = Node<FTMMessage>;

// const isRoot = (message: Message): boolean => {
//   return message.ftm_prev_alt === -1;
// };

export function sortMessageList(messages: FTMMessage[]): FTMMessage[] {
  return messages.slice(0).sort((a, b) => {
    if (a.ftm_num === b.ftm_num) {
      return a.ftm_alt - b.ftm_alt;
    }
    return a.ftm_num - b.ftm_num;
  });
}

export const makeMessageTrie = (
  messages: FTMMessage[],
): FTMMessageNode | EmptyNode => {
  if (messages.length === 0) return { value: null, children: [] };
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

function getChildren(
  parent: FTMMessage,
  messages: FTMMessage[],
): FTMMessageNode[] {
  if (messages.length === 0) return [];
  const rowNumber = parent.ftm_num + 1;
  const [other, siblings] = partition(messages, (m) => {
    return m.ftm_num === rowNumber && m.ftm_prev_alt === parent.ftm_alt;
  });

  return siblings.map((s) => {
    return { value: s, children: getChildren(s, other) };
  });
}
