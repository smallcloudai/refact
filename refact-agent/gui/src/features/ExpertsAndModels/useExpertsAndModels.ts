import { useCallback } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { graphqlQueriesAndMutations } from "../../services/graphql/graphqlThunks";
import { selectCurrentExpert, setExpert } from "./expertsSlice";
import { selectActiveGroup } from "../Teams";

// TODO: move this
export const useExpertsAndModels = () => {
  const dispatch = useAppDispatch();
  const workspace = useAppSelector(selectActiveGroup);
  const selectedExpert = useAppSelector(selectCurrentExpert);
  const expertsQuery = graphqlQueriesAndMutations.useExpertsQuery(
    { located_fgroup_id: workspace?.id ?? "" },
    { skip: !workspace?.id },
  );

  const onSelectExpert = useCallback(
    (expertId: string) => dispatch(setExpert(expertId)),
    [dispatch],
  );

  return {
    experts: expertsQuery.data,
    expertsLoading: expertsQuery.isFetching || expertsQuery.isLoading,
    selectedExpert,
    onSelectExpert,
  };
};
