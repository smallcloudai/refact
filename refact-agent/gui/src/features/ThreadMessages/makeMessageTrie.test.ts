import { expect, describe, test } from "vitest";
import { makeMessageTrie, getAncestorsForNode } from "./makeMessageTrie";
import {
  STUB_ALICE_MESSAGES,
  STUB_BRANCHED_MESSAGES,
} from "../../__fixtures__/message_lists";

describe("makeMessageTree", () => {
  test("thread with no branches", () => {
    const result = makeMessageTrie(STUB_ALICE_MESSAGES);

    expect(result).toEqual({
      value: STUB_ALICE_MESSAGES[0],
      children: [
        {
          value: STUB_ALICE_MESSAGES[1],
          children: [{ value: STUB_ALICE_MESSAGES[2], children: [] }],
        },
      ],
    });
  });

  test("thread with branches", () => {
    const result = makeMessageTrie(STUB_BRANCHED_MESSAGES);

    expect(result).toEqual({
      value: {
        ftm_alt: 100,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content:
          "what are the current plans for human exploration of mars?",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 1,
        ftm_prev_alt: 100,
        ftm_role: "user",
        ftm_tool_calls: null,
        ftm_usage: null,
      },
      children: [
        {
          value: {
            ftm_alt: 100,
            ftm_belongs_to_ft_id: "solarthread1",
            ftm_call_id: "",
            ftm_content:
              "Current Mars exploration plans include NASA's Artemis program as a stepping stone, with a potential human mission in the 2030s. SpaceX has more ambitious timelines with their Starship vehicle. Key challenges include radiation protection, life support systems, and developing in-situ resource utilization for fuel and supplies.",
            ftm_created_ts: 1748611664.270086,
            ftm_num: 2,
            ftm_prev_alt: 100,
            ftm_role: "assistant",
            ftm_tool_calls: null,
            ftm_usage: {
              completion_tokens: 450,
              model: "gpt-4.1-mini",
              prompt_tokens: 100,
            },
          },
          children: [
            {
              value: {
                ftm_alt: 100,
                ftm_belongs_to_ft_id: "solarthread1",
                ftm_call_id: "",
                ftm_content:
                  "what is the typical temperature on Mars, short answer",
                ftm_created_ts: 1748611664.270086,
                ftm_num: 3,
                ftm_prev_alt: 100,
                ftm_role: "user",
                ftm_tool_calls: null,
                ftm_usage: null,
              },
              children: [
                {
                  value: {
                    ftm_alt: 100,
                    ftm_belongs_to_ft_id: "solarthread1",
                    ftm_call_id: "",
                    ftm_content: "cold",
                    ftm_created_ts: 1748611664.270086,
                    ftm_num: 4,
                    ftm_prev_alt: 100,
                    ftm_role: "assistant",
                    ftm_tool_calls: null,
                    ftm_usage: null,
                  },
                  children: [],
                },
              ],
            },
            {
              value: {
                ftm_alt: 101,
                ftm_belongs_to_ft_id: "solarthread1",
                ftm_call_id: "",
                ftm_content: "have you seen mars attacks",
                ftm_created_ts: 1748611664.270086,
                ftm_num: 3,
                ftm_prev_alt: 100,
                ftm_role: "user",
                ftm_tool_calls: null,
                ftm_usage: null,
              },
              children: [
                {
                  value: {
                    ftm_alt: 101,
                    ftm_belongs_to_ft_id: "solarthread1",
                    ftm_call_id: "",
                    ftm_content: "no",
                    ftm_created_ts: 1748611664.270086,
                    ftm_num: 4,
                    ftm_prev_alt: 101,
                    ftm_role: "assistant",
                    ftm_tool_calls: null,
                    ftm_usage: null,
                  },
                  children: [],
                },
              ],
            },
          ],
        },
      ],
    });
  });
});

describe("getAncestorsForNode", () => {
  test("thread with no branches", () => {
    const end = STUB_ALICE_MESSAGES[STUB_ALICE_MESSAGES.length - 1];
    const result = getAncestorsForNode(
      end.ftm_num,
      end.ftm_alt,
      end.ftm_prev_alt,
      STUB_ALICE_MESSAGES,
    );
    expect(result).toEqual(STUB_ALICE_MESSAGES);
  });

  test("thread with branches (branched)", () => {
    const end = STUB_BRANCHED_MESSAGES[STUB_BRANCHED_MESSAGES.length - 1];
    const result = getAncestorsForNode(
      end.ftm_num,
      end.ftm_alt,
      end.ftm_prev_alt,
      STUB_BRANCHED_MESSAGES,
    );
    expect(result).toMatchInlineSnapshot([
      {
        ftm_alt: 100,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content:
          "what are the current plans for human exploration of mars?",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 1,
        ftm_prev_alt: 100,
        ftm_role: "user",
        ftm_tool_calls: null,
        ftm_usage: null,
      },
      {
        ftm_alt: 100,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content:
          "Current Mars exploration plans include NASA's Artemis program as a stepping stone, with a potential human mission in the 2030s. SpaceX has more ambitious timelines with their Starship vehicle. Key challenges include radiation protection, life support systems, and developing in-situ resource utilization for fuel and supplies.",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 2,
        ftm_prev_alt: 100,
        ftm_role: "assistant",
        ftm_tool_calls: null,
        ftm_usage: {
          completion_tokens: 450,
          model: "gpt-4.1-mini",
          prompt_tokens: 100,
        },
      },
      {
        ftm_alt: 101,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content: "have you seen mars attacks",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 3,
        ftm_prev_alt: 100,
        ftm_role: "user",
        ftm_tool_calls: null,
        ftm_usage: null,
      },
      {
        ftm_alt: 101,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content: "no",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 4,
        ftm_prev_alt: 101,
        ftm_role: "assistant",
        ftm_tool_calls: null,
        ftm_usage: null,
      },
    ]);
  });

  test("thread with branches (unbranched)", () => {
    const end = STUB_BRANCHED_MESSAGES[STUB_BRANCHED_MESSAGES.length - 2];
    const result = getAncestorsForNode(
      end.ftm_num,
      end.ftm_alt,
      end.ftm_prev_alt,
      STUB_BRANCHED_MESSAGES,
    );

    expect(result).toMatchInlineSnapshot([
      {
        ftm_alt: 100,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content:
          "what are the current plans for human exploration of mars?",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 1,
        ftm_prev_alt: 100,
        ftm_role: "user",
        ftm_tool_calls: null,
        ftm_usage: null,
      },
      {
        ftm_alt: 100,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content:
          "Current Mars exploration plans include NASA's Artemis program as a stepping stone, with a potential human mission in the 2030s. SpaceX has more ambitious timelines with their Starship vehicle. Key challenges include radiation protection, life support systems, and developing in-situ resource utilization for fuel and supplies.",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 2,
        ftm_prev_alt: 100,
        ftm_role: "assistant",
        ftm_tool_calls: null,
        ftm_usage: {
          completion_tokens: 450,
          model: "gpt-4.1-mini",
          prompt_tokens: 100,
        },
      },
      {
        ftm_alt: 100,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content: "what is the typical temperature on Mars, short answer",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 3,
        ftm_prev_alt: 100,
        ftm_role: "user",
        ftm_tool_calls: null,
        ftm_usage: null,
      },
      {
        ftm_alt: 100,
        ftm_belongs_to_ft_id: "solarthread1",
        ftm_call_id: "",
        ftm_content: "cold",
        ftm_created_ts: 1748611664.270086,
        ftm_num: 4,
        ftm_prev_alt: 100,
        ftm_role: "assistant",
        ftm_tool_calls: null,
        ftm_usage: null,
      },
    ]);
  });
});
