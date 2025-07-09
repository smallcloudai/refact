import { createSlice, PayloadAction } from "@reduxjs/toolkit/react";
import { WorkspaceTreeSubscription } from "../../../generated/documents";
import { workspaceTreeSubscriptionThunk } from "../../services/graphql/graphqlThunks";
import {
  cleanupInsertedLater,
  markForDelete,
  pruneNodes,
  updateTree,
  type FlexusTreeNode,
} from "./utils";

type InitialState = {
  loading: boolean;
  error: string | null;
  // TODO: move flexusTreeNode
  data: FlexusTreeNode[];
  finished: boolean;
};
const initialState: InitialState = {
  loading: false,
  error: null,
  finished: false,
  data: [],
};
export const groupsSlice = createSlice({
  name: "groups",
  initialState,
  reducers: {
    receiveWorkspace: (
      state,
      action: PayloadAction<WorkspaceTreeSubscription["tree_subscription"]>,
    ) => {
      if (action.payload.treeupd_action === "TREE_REBUILD_START") {
        state.loading = true;
        state.finished = false;
        const data = markForDelete(state.data);
        state.data = data;
      }
      if (action.payload.treeupd_action === "TREE_REBUILD_FINISHED") {
        state.loading = false;
        state.finished = true;
      }

      if (
        action.payload.treeupd_action === "TREE_UPDATE" &&
        action.payload.treeupd_path
      ) {
        // touch node + update tree
        // state.data[action.payload.treeupd_id] = action.payload;
        const parts = action.payload.treeupd_path.split("/");
        const next = updateTree(
          state.data,
          parts,
          "",
          action.payload.treeupd_id,
          action.payload.treeupd_path,
          action.payload.treeupd_title,
          action.payload.treeupd_type,
        );
        state.data = next;
      }
      // state.data[action.payload.treeupd_id] = action.payload;
    },
    receiveWorkspaceError: (state, action: PayloadAction<string>) => {
      state.loading = false;
      state.error = action.payload;
    },

    // markWorkspacesForDelete: (state) => {
    //   const data = markForDelete(state.data);
    //   state.data = data;
    // },

    pruneWorkspaceNodes: (state) => {
      const data = pruneNodes(state.data);
      state.data = data;
    },

    cleanupWorkspaceInsertedLater: (state) => {
      const data = cleanupInsertedLater(state.data);
      state.data = data;
    },
  },
  extraReducers(builder) {
    builder.addCase(workspaceTreeSubscriptionThunk.pending, (state) => {
      state.loading = true;
    });
  },

  selectors: {
    selectWorkspaceState: (state) => state,
    selectWorkspacesTree: (state) => state.data,
    selectWorkspacesLoading: (state) => state.loading,
    selectWorkspacesFinished: (state) => state.finished,
    selectWorkspacesError: (state) => state.error,
  },
});

export const {
  receiveWorkspace,
  receiveWorkspaceError,
  pruneWorkspaceNodes,
  cleanupWorkspaceInsertedLater,
} = groupsSlice.actions;
export const { selectWorkspaceState } = groupsSlice.selectors;
