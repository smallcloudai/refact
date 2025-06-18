import React, { useCallback, useEffect, useMemo } from "react";
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
import { Skeleton } from "@radix-ui/themes";
import { Select } from "../../components/Select";
import { selectActiveGroup } from "../Teams";

const useExpertsAndModels = () => {
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

export const ExpertSelect: React.FC<{ disabled?: boolean }> = ({
  disabled,
}) => {
  const { experts, expertsLoading, selectedExpert, onSelectExpert } =
    useExpertsAndModels();

  // should be handled in the slice
  const selectedExpertOrDefault = useMemo(() => {
    if (selectedExpert) return selectedExpert;
    if (experts.length > 0) return experts[0].fexp_id;
    return undefined;
  }, [experts, selectedExpert]);

  return (
    <Skeleton loading={expertsLoading}>
      <Select
        disabled={disabled}
        onChange={onSelectExpert}
        value={selectedExpertOrDefault}
        options={experts.map((expert) => ({
          value: expert.fexp_id,
          textValue: expert.fexp_name,
        }))}
      />
    </Skeleton>
  );
};
