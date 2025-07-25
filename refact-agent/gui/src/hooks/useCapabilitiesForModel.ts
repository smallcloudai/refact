import { useMemo } from "react";
import {
  selectCurrentExpert,
  selectCurrentModel,
} from "../features/ExpertsAndModels/expertsSlice";
import { selectActiveGroup } from "../features/Teams/teamsSlice";
import { graphqlQueriesAndMutations } from "../services/graphql/queriesAndMutationsApi";
import { useAppSelector } from "./useAppSelector";

type ModelCapabilities = {
  modelcaps_input_images?: boolean;
  modelcaps_reasoning_effort?: boolean;
};
function isModelCapabilities(obj: unknown): obj is ModelCapabilities {
  if (!obj) return false;
  if (typeof obj !== "object") return false;
  if (
    "modelcaps_input_images" in obj &&
    typeof obj.modelcaps_input_images !== "boolean"
  ) {
    return false;
  }
  if (
    "modelcaps_reasoning_effort" in obj &&
    typeof obj.modelcaps_reasoning_effort !== "boolean"
  ) {
    return false;
  }
  return true;
}

export function useCapabilitiesForModel() {
  const selectedExpert = useAppSelector(selectCurrentExpert);
  const selectedModel = useAppSelector(selectCurrentModel);
  const workspace = useAppSelector(selectActiveGroup);
  const expertsQuery = graphqlQueriesAndMutations.useModelsForExpertQuery(
    { inside_fgroup_id: workspace?.id ?? "", fexp_id: selectedExpert ?? "" },
    { skip: !workspace?.id || !selectedExpert },
  );

  const capsForModel = useMemo<ModelCapabilities>(() => {
    const model = expertsQuery.data?.expert_choice_consequences.models.find(
      (model) => model.provm_name === selectedModel,
    );

    if (isModelCapabilities(model?.provm_caps)) {
      return model.provm_caps;
    }
    return {
      modelcaps_input_images: false,
      modelcaps_reasoning_effort: false,
    };
  }, [expertsQuery.data?.expert_choice_consequences.models, selectedModel]);

  return {
    multimodal: capsForModel.modelcaps_input_images ?? false,
    thinking: capsForModel.modelcaps_reasoning_effort ?? false,
    loading: expertsQuery.isFetching || expertsQuery.isFetching,
  };
}
