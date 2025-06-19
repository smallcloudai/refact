import { useEffect, useMemo } from "react";
import { getToolsForGroupThunk } from "../../services/graphql/graphqlThunks";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectActiveGroup } from "../Teams";
import { selectToolsForGroups, selectToolsLoading } from "./toolsSlice";

export function useToolsForGroup() {
  const dispatch = useAppDispatch();
  const group = useAppSelector(selectActiveGroup);
  const loading = useAppSelector(selectToolsLoading);
  const toolsForGroups = useAppSelector(selectToolsForGroups);

  useEffect(() => {
    if (group?.id) {
      void dispatch(getToolsForGroupThunk({ located_fgroup_id: group.id }));
    }
  }, [dispatch, group?.id]);

  const isLoading = useMemo(() => {
    if (!group?.id) return false;
    if (group.id in toolsForGroups) return false;
    return loading;
  }, [group?.id, loading, toolsForGroups]);

  const toolsForGroup = useMemo(() => {
    if (!group?.id) return [];
    if (group.id in toolsForGroups) return toolsForGroups[group.id];
    return [];
  }, [group?.id, toolsForGroups]);

  return { toolsForGroup, isLoading };
}
