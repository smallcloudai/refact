import { partition } from "../../utils";
// import { MessagesSubscriptionSubscription } from "../../../generated/documents";
import type {
  // ChatMessage,
  BaseMessage,
} from "../../services/refact/types";

// export type FTMMessage = NonNullable<
//   MessagesSubscriptionSubscription["comprehensive_thread_subs"]["news_payload_thread_message"]
// >;

interface Node<T> {
  value: T;
  children: Node<T>[];
}

export type EmptyNode = Node<null>;

export type FTMMessageNode = Node<BaseMessage>;

export function isEmptyNode(
  node: EmptyNode | FTMMessageNode,
): node is EmptyNode {
  return node.value === null;
}

// const isRoot = (message: Message): boolean => {
//   return message.ftm_prev_alt === -1;
// };

export function sortMessageList(messages: BaseMessage[]): BaseMessage[] {
  return messages.slice(0).sort((a, b) => {
    if (a.ftm_num === b.ftm_num) {
      return a.ftm_alt - b.ftm_alt;
    }
    return a.ftm_num - b.ftm_num;
  });
}

export const makeMessageTrie = (
  messages: BaseMessage[],
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
  parent: BaseMessage,
  messages: BaseMessage[],
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

export function getAncestorsForNode(
  num: number,
  alt: number,
  prevAlt: number,
  messages: BaseMessage[],
): BaseMessage[] {
  // TODO: dummy node might cause this to be off by one.
  const child =
    messages.find(
      (message) =>
        message.ftm_num === num &&
        message.ftm_alt === alt &&
        message.ftm_prev_alt === message.ftm_prev_alt,
    ) ?? findParent(num, prevAlt, messages);

  if (!child) return [];
  return getParentsIter(child, messages);
}

function getParentsIter(
  child: BaseMessage,
  messages: BaseMessage[],
  memo: BaseMessage[] = [],
) {
  const maybeParent = findParent(child.ftm_num, child.ftm_prev_alt, messages);
  const collected = [child, ...memo];
  if (!maybeParent) return collected;

  return getParentsIter(maybeParent, messages, collected);
}

function findParent(
  num: number,
  prevAlt: number,
  messages: BaseMessage[],
): BaseMessage | undefined {
  return messages.find((message) => {
    return message.ftm_num === num - 1 && message.ftm_alt === prevAlt;
  });
}
