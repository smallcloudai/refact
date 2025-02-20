import { expect, describe, test } from "vitest";
import { CMessage } from "../../services/refact";
import { CMessageNode, CMessageRoot } from "./chatDbMessagesSlice";
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
    cmessage_alt: 0,
    cmessage_num: 2,
    cmessage_prev_alt: 1,
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
    cmessage_num: 3,
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
    cmessage_num: 4,
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
    cmessage_num: 5,
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
    cmessage_num: 6,
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

const EXPECTED: CMessageRoot[] = [
  [
    {
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
    },
  ],
];

describe("makeMessageTree", () => {
  test("should return a tree", () => {
    const tree = makeMessageTree(STUB);
    console.dir({ tree }, { depth: 1000 });
    expect(tree).toEqual(EXPECTED);
  });
});

function makeMessageTree(messages: CMessage[]): CMessageRoot[] {
  // Create a map to store messages by their num and alt combination
  const messageMap = new Map<string, CMessageNode>();

  // First pass: create map entries for all messages
  messages.forEach((message) => {
    const key = `${message.cmessage_num}_${message.cmessage_alt}`;
    messageMap.set(key, {
      message,
      children: [],
    });
  });

  // Second pass: build relationships
  messages.forEach((message) => {
    if (message.cmessage_prev_alt >= 0) {
      const currentKey = `${message.cmessage_num}_${message.cmessage_alt}`;
      const currentNode = messageMap.get(currentKey);
      if (!currentNode) return;

      const parentKey = `${message.cmessage_prev_alt}_${message.cmessage_alt}`;
      const parentNode = messageMap.get(parentKey);
      if (parentNode) {
        parentNode.children.push(currentNode);
      }
    }
  });

  // Find root messages (those with cmessage_prev_alt === -1)
  const roots: CMessageRoot[] = [];
  messages.forEach((message) => {
    if (message.cmessage_prev_alt === -1) {
      const key = `${message.cmessage_num}_${message.cmessage_alt}`;
      const node = messageMap.get(key);
      if (node) {
        roots.push([node]);
      }
    }
  });

  return roots;
}

interface CMessageNode {
  message: CMessage;
  children: CMessageNode[];
}

// Adding type definition for clarity
interface CMessageNode {
  message: CMessage;
  children: CMessageNode[];
}
