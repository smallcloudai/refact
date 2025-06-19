import { useEffect, useMemo } from "react";
import { getToolsForGroupThunk } from "../../services/graphql/graphqlThunks";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectActiveGroup } from "../Teams";
import { selectToolsForGroup, selectToolsLoading } from "./toolsSlice";

export function useToolsForGroup() {
  const dispatch = useAppDispatch();
  const group = useAppSelector(selectActiveGroup);
  const loading = useAppSelector(selectToolsLoading);
  const toolsForGroup = useAppSelector(selectToolsForGroup);

  useEffect(() => {
    if (group?.id) {
      void dispatch(getToolsForGroupThunk({ located_fgroup_id: group.id }));
    }
  }, [dispatch, group?.id]);

  const isLoading = useMemo(() => {
    if (!group?.id) return false;
    return loading;
  }, [group?.id, loading]);

  return { toolsForGroup, isLoading };
}
