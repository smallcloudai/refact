import { useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
// import { useCapsForToolUse } from "./useCapsForToolUse";
import { selectActiveGroup } from "../features/Teams";

/**
 * Use this hook to get states related to caps supported features alongside the current active teams group.
 **/
// TODO: do we keep this?
export function useActiveTeamsGroup() {
  // const { data: capsData } = useCapsForToolUse();
  const maybeActiveTeamsGroup = useAppSelector(selectActiveGroup);
  const isKnowledgeFeatureAvailable = true;
  // const isKnowledgeFeatureAvailable = useMemo(() => {
  //   return capsData?.metadata?.features?.includes("knowledge") === true;
  // }, [capsData?.metadata?.features]);

  const groupSelectionEnabled = useMemo(() => {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    return isKnowledgeFeatureAvailable && !maybeActiveTeamsGroup;
  }, [maybeActiveTeamsGroup, isKnowledgeFeatureAvailable]);

  const newChatEnabled = useMemo(() => {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    if (isKnowledgeFeatureAvailable) {
      return !!maybeActiveTeamsGroup;
    }

    return true;
  }, [maybeActiveTeamsGroup, isKnowledgeFeatureAvailable]);

  return {
    groupSelectionEnabled,
    isKnowledgeFeatureAvailable,
    newChatEnabled,
  };
}
