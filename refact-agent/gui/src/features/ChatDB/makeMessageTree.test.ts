import { expect, describe, test } from "vitest";
import { CMessage } from "../../services/refact";
import { CMessageNode } from "./chatDbMessagesSlice";
import { makeMessageTree } from "./makeMessageTrie";
import { CMESSAGES_STUB } from "../../__fixtures__";

const STUB = CMESSAGES_STUB;

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
    const input = [STUB[0], STUB[1], STUB[2], STUB[3], STUB[4]];
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
        { message: input[2], children: [{ message: input[4], children: [] }] },
      ],
    });
  });

  test("full tries and replies", () => {
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
                  message: STUB[5],
                  children: [
                    {
                      message: STUB[6],
                      children: [],
                    },
                  ],
                },
              ],
            },
          ],
        },
        { message: STUB[2], children: [{ message: STUB[4], children: [] }] },
      ],
    });
  });

  test("tries from tires", () => {
    const input: CMessage[] = [
      STUB[0],
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
          content: "Hello",
        },
      },
      {
        cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
        cmessage_alt: 0,
        cmessage_num: 2,
        cmessage_prev_alt: 0,
        cmessage_usage_model: "",
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: {
          role: "assistant",
          content: "Hello.",
        },
      },
      {
        cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
        cmessage_alt: 0,
        cmessage_num: 3,
        cmessage_prev_alt: 0,
        cmessage_usage_model: "",
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: {
          role: "user",
          content: "1",
        },
      },
      {
        cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
        cmessage_alt: 1,
        cmessage_num: 3,
        cmessage_prev_alt: 0,
        cmessage_usage_model: "",
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: {
          role: "user",
          content: "2",
        },
      },
      {
        cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
        cmessage_alt: 0,
        cmessage_num: 4,
        cmessage_prev_alt: 0,
        cmessage_usage_model: "",
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: {
          role: "assistant",
          content: "1",
        },
      },
      {
        cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
        cmessage_alt: 1,
        cmessage_num: 4,
        cmessage_prev_alt: 1,
        cmessage_usage_model: "",
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: {
          role: "assistant",
          content: "2",
        },
      },
      {
        cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
        cmessage_alt: 0,
        cmessage_num: 5,
        cmessage_prev_alt: 1,
        cmessage_usage_model: "",
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: {
          role: "user",
          content: "4",
        },
      },
      {
        cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
        cmessage_alt: 0,
        cmessage_num: 6,
        cmessage_prev_alt: 0,
        cmessage_usage_model: "",
        cmessage_usage_prompt: 0,
        cmessage_usage_completion: 0,
        cmessage_json: {
          role: "assistant",
          content: "🏌️",
        },
      },
    ];

    const tree = makeMessageTree(input);

    const expected: CMessageNode = {
      message: input[0],
      children: [
        {
          message: input[1],
          children: [
            {
              message: input[2],
              children: [
                {
                  message: input[3],
                  children: [{ message: input[5], children: [] }],
                },
                {
                  message: input[4],
                  children: [
                    {
                      message: input[6],
                      children: [
                        {
                          message: input[7],
                          children: [{ message: input[8], children: [] }],
                        },
                      ],
                    },
                  ],
                },
              ],
            },
          ],
        },
      ],
    };

    expect(tree).toEqual(expected);
  });
});
