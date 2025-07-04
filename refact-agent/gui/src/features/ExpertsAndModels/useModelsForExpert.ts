import { useCallback, useEffect, useMemo } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { getModelsForExpertThunk } from "../../services/graphql/graphqlThunks";
import {
  selectCurrentExpert,
  selectCurrentModel,
  selectModelsForExpert,
  selectModelsForExpertLoading,
  setModel,
} from "./expertsSlice";

import { selectActiveGroup } from "../Teams";

export const useModelsForExpert = () => {
  const dispatch = useAppDispatch();
  const workspace = useAppSelector(selectActiveGroup);
  const selectedExpert = useAppSelector(selectCurrentExpert);
  const modelsForExpert = useAppSelector(selectModelsForExpert, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const modelsLoading = useAppSelector(selectModelsForExpertLoading);
  const selectedModel = useAppSelector(selectCurrentModel);

  useEffect(() => {
    if (selectedExpert && workspace?.id) {
      void dispatch(
        getModelsForExpertThunk({
          fexp_id: selectedExpert,
          inside_fgroup_id: workspace.id,
        }),
      );
    }
  }, [dispatch, selectedExpert, workspace?.id]);

  const selectModel = useCallback(
    (value: string) => dispatch(setModel(value)),
    [dispatch],
  );

  const selectedModelOrDefault = useMemo(() => {
    if (selectedModel) return selectedModel;
    // if (modelsForExpert.length > 0) return modelsForExpert[0].provm_name;
    return undefined;
  }, [selectedModel]);

  const options = useMemo(() => {
    return modelsForExpert.map((model) => model.provm_name);
  }, [modelsForExpert]);

  return {
    modelsLoading,
    selectedModelOrDefault,
    selectModel,
    options,
  };
};
