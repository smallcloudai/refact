import { CMessage, CMessageFromChatDB } from "../services/refact/types";

export const CMESSAGES_STUB: CMessage[] = [
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
      role: "assistant",
      content: "yes?.",
    },
  },
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 1,
    cmessage_num: 2,
    cmessage_prev_alt: 1,
    cmessage_usage_model: "gpt-4o-mini",
    cmessage_usage_prompt: 0,
    cmessage_usage_completion: 0,
    cmessage_json: {
      role: "assistant",
      content: "Birds aren't real",
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
      content: "Find Frog in this project",
    },
  },
  {
    cmessage_belongs_to_cthread_id: "test13thread1739988322_2",
    cmessage_alt: 0,
    cmessage_num: 4,
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

export const CSMESSAGES_NETWORK_STUB: CMessageFromChatDB[] = CMESSAGES_STUB.map(
  (cmessage) => {
    return {
      ...cmessage,
      cmessage_json: JSON.stringify(cmessage.cmessage_json),
    };
  },
);

export const CMESSAGES_WITH_NESTED_BRANCHES_STUB: CMessage[] = [
  CMESSAGES_STUB[0],
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
