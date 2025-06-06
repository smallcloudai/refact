import { FTMMessage } from "../features/ThreadMessages/makeMessageTrie";

export const STUB_ALICE_MESSAGES: FTMMessage[] = [
  {
    ftm_belongs_to_ft_id: "solarthread1",
    ftm_alt: 100,
    ftm_num: 1,
    ftm_prev_alt: 100,
    ftm_role: "user",
    ftm_content: "what are the current plans for human exploration of mars?",
    ftm_tool_calls: null,
    ftm_call_id: "",
    ftm_usage: null,
    ftm_created_ts: 1748611664.270086,
  },
  {
    ftm_belongs_to_ft_id: "solarthread1",
    ftm_alt: 100,
    ftm_num: 2,
    ftm_prev_alt: 100,
    ftm_role: "assistant",
    ftm_content:
      "Current Mars exploration plans include NASA's Artemis program as a stepping stone, with a potential human mission in the 2030s. SpaceX has more ambitious timelines with their Starship vehicle. Key challenges include radiation protection, life support systems, and developing in-situ resource utilization for fuel and supplies.",
    ftm_tool_calls: null,
    ftm_call_id: "",
    ftm_usage: {
      model: "gpt-4.1-mini",
      prompt_tokens: 100,
      completion_tokens: 450,
    },
    ftm_created_ts: 1748611664.270086,
  },
  {
    ftm_belongs_to_ft_id: "solarthread1",
    ftm_alt: 100,
    ftm_num: 3,
    ftm_prev_alt: 100,
    ftm_role: "user",
    ftm_content: "what is the typical temperature on Mars, short answer",
    ftm_tool_calls: null,
    ftm_call_id: "",
    ftm_usage: null,
    ftm_created_ts: 1748611664.270086,
  },
];
