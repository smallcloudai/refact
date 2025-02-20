import { expect, describe, test } from "vitest";
import { CMessage } from "../../services/refact";
import { CMessageNode } from "./chatDbMessagesSlice";
import { partition } from "../../utils";
const STUB: CMessage[] = [
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 0,
    cmessage_num: 0,
    cmessage_prev_alt: -1,
    cmessage_usage_model: "",
    cmessage_usage_prompt: 0,
    cmessage_usage_completion: 0,
    cmessage_json: {
      role: "system",
      content: "You answer only with jokes.",
    },
  },
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 0,
    cmessage_num: 1,
    cmessage_prev_alt: 0,
    cmessage_usage_model: "",
    cmessage_usage_prompt: 0,
    cmessage_usage_completion: 0,
    cmessage_json: {
      role: "user",
      content: "Hello mister assistant, I have a question for you",
    },
  },
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 1,
    cmessage_num: 1,
    cmessage_prev_alt: 0,
    cmessage_usage_model: "",
    cmessage_usage_prompt: 0,
    cmessage_usage_completion: 0,
    cmessage_json: {
      role: "user",
      content: "Find Frog in this project",
    },
  },
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 0,
    cmessage_num: 2,
    cmessage_prev_alt: 0,
    cmessage_usage_model: "gpt-4o-mini",
    cmessage_usage_prompt: 0,
    cmessage_usage_completion: 0,
    cmessage_json: {
      role: "system",
      content: "You answer only with jokes.",
    },
  },
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 0,
    cmessage_num: 3,
    cmessage_prev_alt: 0,
    cmessage_usage_model: "gpt-4o-mini",
    cmessage_usage_prompt: 0,
    cmessage_usage_completion: 0,
    cmessage_json: {
      role: "user",
      content: "Hello mister assistant, I have a question for you",
    },
  },
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 0,
    cmessage_num: 4,
    cmessage_prev_alt: 0,
    cmessage_usage_model: "gpt-4o-mini",
    cmessage_usage_prompt: 0,
    cmessage_usage_completion: 0,
    cmessage_json: {
      role: "user",
      content: "Find Frog in this project",
    },
  },
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 0,
    cmessage_num: 5,
    cmessage_prev_alt: 0,
    cmessage_usage_model: "gpt-4o-mini",
    cmessage_usage_prompt: 1210,
    cmessage_usage_completion: 15,
    cmessage_json: {
      role: "assistant",
      content: "",
      tool_calls: [
        {
          index: 0,
          id: "call_8PSEh32Hhivfdxc50XKNwW8y",
          function: {
            arguments: '{"symbol":"Frog"}',
            name: "references",
          },
          type: "function",
        },
      ],
      //   usage: {
      //     prompt_tokens: 1210,
      //     completion_tokens: 15,
      //     total_tokens: 1225,
      //   },
    },
  },
];

describe("makeMessageTree", () => {
  test("no root", () => {
    const tree = makeMessageTree([STUB[1], STUB[2]]);
    expect(tree).toEqual(null);
  });

  test("only root", () => {
    const tree = makeMessageTree([STUB[0]]);
    expect(tree).toEqual({
      message: STUB[0],
      children: [],
    });
  });

  test("root with one child", () => {
    const input = [STUB[0], STUB[1]];
    const tree = makeMessageTree(input);
    expect(tree).toEqual({
      message: input[0],
      children: [
        {
          message: input[1],
          children: [],
        },
      ],
    });
  });

  test("root with two children", () => {
    const input = [STUB[0], STUB[1], STUB[2]];
    const tree = makeMessageTree(input);
    // console.dir({ tree }, { depth: null });
    expect(tree).toEqual({
      message: input[0],
      children: [
        {
          message: input[1],
          children: [],
        },
        { message: input[2], children: [] },
      ],
    });
  });

  test("root with nested children", () => {
    const input = [STUB[0], STUB[1], STUB[2], STUB[3]];
    const tree = makeMessageTree(input);
    expect(tree).toEqual({
      message: input[0],
      children: [
        {
          message: input[1],
          children: [
            {
              message: input[3],
              children: [],
            },
          ],
        },
        { message: input[2], children: [] },
      ],
    });
  });

  test("full stub", () => {
    const tree = makeMessageTree(STUB);
    expect(tree).toEqual({
      message: STUB[0],
      children: [
        {
          message: STUB[1],
          children: [
            {
              message: STUB[3],
              children: [
                {
                  message: STUB[4],
                  children: [
                    {
                      message: STUB[5],
                      children: [{ message: STUB[6], children: [] }],
                    },
                  ],
                },
              ],
            },
          ],
        },
        { message: STUB[2], children: [] },
      ],
    });
  });
});

const isRoot = (message: CMessage): boolean => {
  return message.cmessage_prev_alt === -1;
};

const makeMessageTree = (messages: CMessage[]): CMessageNode | null => {
  const sortedMessages = messages
    .slice()
    .sort((a, b) => a.cmessage_num - b.cmessage_num);

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
  const [other, siblings] = partition(
    messages,
    (m) => m.cmessage_num === parent.cmessage_num + 1,
  );

  const children = siblings
    .filter((m) => {
      return m.cmessage_prev_alt === parent.cmessage_alt;
    })
    .map((m) => {
      return { message: m, children: getChildren(m, other) };
    });

  return children;
}
