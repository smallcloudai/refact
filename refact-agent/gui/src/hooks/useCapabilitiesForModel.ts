import { useMemo } from "react";
import {
  selectCurrentExpert,
  selectCurrentModel,
} from "../features/ExpertsAndModels/expertsSlice";
import { selectActiveGroup } from "../features/Teams/teamsSlice";
import { graphqlQueriesAndMutations } from "../services/graphql/queriesAndMutationsApi";
import { useAppSelector } from "./useAppSelector";

export function useCapabilitiesForModel() {
  const selectedExpert = useAppSelector(selectCurrentExpert);
  const selectedModel = useAppSelector(selectCurrentModel);
  const workspace = useAppSelector(selectActiveGroup);
  const expertsQuery = graphqlQueriesAndMutations.useModelsForExpertQuery(
    { inside_fgroup_id: workspace?.id ?? "", fexp_id: selectedExpert ?? "" },
    { skip: !workspace?.id || !selectedExpert },
  );

  const capsForModel = useMemo(() => {
    return (
      expertsQuery.data?.expert_choice_consequences.models.find(
        (model) => model.provm_name === selectedModel,
      )?.provm_caps ?? { input_images: false, reasoning_effort: false }
    );
  }, [expertsQuery.data?.expert_choice_consequences.models, selectedModel]);

  const multimodal = capsForModel.input_images;
  const thinking = capsForModel.reasoning_effort;

  return {
    multimodal,
    thinking,
    loading: expertsQuery.isFetching || expertsQuery.isFetching,
  };
}
