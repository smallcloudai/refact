import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import type {
  TeamsWorkspace,
  TeamsGroup,
} from "../../services/smallcloud/types";

export type TeamsSliceState = {
  group: TeamsGroup | null;
  workspace: Partial<TeamsWorkspace> | null;
  skipped: boolean;
};

const initialState: TeamsSliceState = {
  group: null,
  workspace: null,
  skipped: false,
};

export const teamsSlice = createSlice({
  name: "teams",
  initialState: initialState,
  reducers: {
    setActiveGroup: (state, action: PayloadAction<TeamsGroup>) => {
      state.group = action.payload;
    },
    setSkippedWorkspaceSelection: (state, action: PayloadAction<boolean>) => {
      state.skipped = action.payload;
    },
    setActiveWorkspace: (state, action: PayloadAction<TeamsWorkspace>) => {
      state.workspace = action.payload;
    },
    resetActiveGroup: (state) => {
      state.group = null;
    },
    resetActiveWorkspace: (state) => {
      state.workspace = null;
    },
  },
  selectors: {
    selectActiveGroup: (state) => state.group,
    selectActiveWorkspace: (state) => state.workspace,
    selectIsSkippedWorkspaceSelection: (state) => state.skipped,
  },
});

export const {
  selectIsSkippedWorkspaceSelection,
  selectActiveGroup,
  selectActiveWorkspace,
} = teamsSlice.selectors;
export const {
  setActiveGroup,
  setActiveWorkspace,
  setSkippedWorkspaceSelection,
  resetActiveGroup,
  resetActiveWorkspace,
} = teamsSlice.actions;
