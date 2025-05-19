import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { type Workspace } from "../../services/smallcloud/types";

export type ImageFile = {
  name: string;
  content: string | ArrayBuffer | null;
  type: string;
};

const initialState: { workspace: Workspace | null } = {
  workspace: null,
};

export const activeWorkspaceSlice = createSlice({
  name: "activeWorkspace",
  initialState: initialState,
  reducers: {
    setActiveWorkspace: (state, action: PayloadAction<Workspace>) => {
      state.workspace = action.payload;
    },
    resetActiveWorkspace: () => {
      return initialState;
    },
  },
  selectors: {
    selectActiveWorkspace: (state) => state.workspace,
  },
});

export const { selectActiveWorkspace } = activeWorkspaceSlice.selectors;
export const { setActiveWorkspace, resetActiveWorkspace } =
  activeWorkspaceSlice.actions;
