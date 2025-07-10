import { FThreadMessageInput } from "../../../generated/documents";
import { graphqlQueriesAndMutations } from "./queriesAndMutationsApi";

export function rejectToolUsageAction(
  ids: string[],
  ft_id: string,
  endNumber: number,
  endAlt: number,
  endPrevAlt: number,
) {
  const messagesToSend: FThreadMessageInput[] = ids.map((id, index) => {
    return {
      ftm_role: "tool",
      ftm_belongs_to_ft_id: ft_id,
      ftm_content: JSON.stringify("The user rejected the changes."),
      ftm_call_id: id,
      ftm_num: endNumber + index + 1,
      ftm_alt: endAlt,
      ftm_prev_alt: endPrevAlt,
      ftm_provenance: "null",
    };
  });

  const action = graphqlQueriesAndMutations.endpoints.sendMessages.initiate({
    input: { messages: messagesToSend, ftm_belongs_to_ft_id: ft_id },
  });

  return action;
}
