import { useCallback, useEffect } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import {
  getExpertsThunk,
  getModelsForExpertThunk,
} from "../../services/graphql/graphqlThunks";
import {
  selectAvailableExperts,
  selectCurrentExpert,
  selectIsExpertsLoading,
  setExpert,
} from "./expertsSlice";
import { selectActiveGroup } from "../Teams";

// TODO: move this
export const useExpertsAndModels = () => {
  const dispatch = useAppDispatch();
  const workspace = useAppSelector(selectActiveGroup);
  const selectedExpert = useAppSelector(selectCurrentExpert);
  const expertsLoading = useAppSelector(selectIsExpertsLoading);
  const experts = useAppSelector(selectAvailableExperts);

  const onSelectExpert = useCallback(
    (expertId: string) => dispatch(setExpert(expertId)),
    [dispatch],
  );

  useEffect(() => {
    if (workspace?.id) {
      void dispatch(getExpertsThunk({ located_fgroup_id: workspace.id }));
    }
  }, [dispatch, workspace?.id]);

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

  return {
    experts,
    expertsLoading,
    selectedExpert,
    onSelectExpert,
  };
};
