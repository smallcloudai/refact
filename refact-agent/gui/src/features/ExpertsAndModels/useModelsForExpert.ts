import { useCallback, useMemo } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { graphqlQueriesAndMutations } from "../../services/graphql/graphqlThunks";
import {
  selectCurrentExpert,
  selectCurrentModel,
  setModel,
} from "./expertsSlice";

import { selectActiveGroup } from "../Teams";

export const useModelsForExpert = () => {
  const dispatch = useAppDispatch();
  const workspace = useAppSelector(selectActiveGroup);
  const selectedExpert = useAppSelector(selectCurrentExpert);
  const selectedModel = useAppSelector(selectCurrentModel);

  const modelsForExpertRequest =
    graphqlQueriesAndMutations.useModelsForExpertQuery(
      {
        fexp_id: selectedExpert ?? "",
        inside_fgroup_id: workspace?.id ?? "",
      },
      { skip: !workspace?.id || !selectedExpert },
    );

  const selectModel = useCallback(
    (value: string) => dispatch(setModel(value)),
    [dispatch],
  );

  const options = useMemo(() => {
    if (!modelsForExpertRequest.data) return [];
    return modelsForExpertRequest.data.expert_choice_consequences.map(
      (model) => model.provm_name,
    );
  }, [modelsForExpertRequest.data]);

  return {
    modelsLoading:
      modelsForExpertRequest.isFetching || modelsForExpertRequest.isLoading,
    selectedModel: selectedModel,
    selectModel,
    options,
  };
};
