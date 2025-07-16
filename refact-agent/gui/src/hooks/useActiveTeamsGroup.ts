import { useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import {
  selectActiveGroup,
  selectIsSkippedWorkspaceSelection,
} from "../features/Teams/teamsSlice";

/**
 * Use this hook to get states related to caps supported features alongside the current active teams group.
 **/
// TODO: do we keep this?
export function useActiveTeamsGroup() {
  const maybeActiveTeamsGroup = useAppSelector(selectActiveGroup);
  const isWorkspaceSelectionSkipped = useAppSelector(
    selectIsSkippedWorkspaceSelection,
  );
  const groupSelectionEnabled = useMemo(() => {
    if (isWorkspaceSelectionSkipped) return false;
    return !maybeActiveTeamsGroup;
  }, [maybeActiveTeamsGroup, isWorkspaceSelectionSkipped]);

  const newChatEnabled = useMemo(() => {
    if (isWorkspaceSelectionSkipped) return true;
    return !!maybeActiveTeamsGroup;
  }, [maybeActiveTeamsGroup, isWorkspaceSelectionSkipped]);

  return {
    groupSelectionEnabled,
    newChatEnabled,
  };
}
