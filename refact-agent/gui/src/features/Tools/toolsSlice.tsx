import { createSlice } from "@reduxjs/toolkit/react";
import { ToolsForGroupQuery } from "../../../generated/documents";
import { getToolsForGroupThunk } from "../../services/graphql/graphqlThunks";

type InitialState = {
  loading: boolean;
  error: string | null;
  toolsForGroups: Record<string, ToolsForGroupQuery["cloud_tools_list"]>;
};

const initialState: InitialState = {
  loading: false,
  error: null,
  toolsForGroups: {},
};

// TODO: allow the user to configure tools, before and after creating
export const toolsSlice = createSlice({
  name: "toolsForGroup",
  initialState,
  reducers: {},
  selectors: {
    // selectToolsForGroup: (state) => {
    //   if (!(workspace in state.toolsForGroup)) return null;
    //   return state.toolsForGroup[workspace];
    // },
    selectToolsForGroups: (state) => state.toolsForGroups,
    selectToolsLoading: (state) => state.loading,
  },
  extraReducers(builder) {
    builder.addCase(getToolsForGroupThunk.pending, (state) => {
      state.loading = true;
    });
    builder.addCase(getToolsForGroupThunk.rejected, (state, action) => {
      state.loading = false;
      state.error = action.error.message ?? null;
    });

    builder.addCase(getToolsForGroupThunk.fulfilled, (state, action) => {
      state.loading = false;
      state.toolsForGroups[action.meta.arg.located_fgroup_id] =
        action.payload.cloud_tools_list;
    });
  },
});

export const { selectToolsForGroups, selectToolsLoading } =
  toolsSlice.selectors;
