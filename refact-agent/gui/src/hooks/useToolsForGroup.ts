import { graphqlQueriesAndMutations } from "../services/graphql";
import { useAppSelector } from "./useAppSelector";
import { selectActiveGroup } from "../features/Teams/teamsSlice";

export function useToolsForGroup() {
  const group = useAppSelector(selectActiveGroup);
  const toolsForGroupRequest =
    graphqlQueriesAndMutations.useToolsForWorkspaceQuery(
      { located_fgroup_id: group?.id ?? "" },
      { skip: !group?.id },
    );

  return {
    toolsForGroup: toolsForGroupRequest.data?.cloud_tools_list ?? [],
    isLoading:
      toolsForGroupRequest.isFetching || toolsForGroupRequest.isLoading,
  };
}
