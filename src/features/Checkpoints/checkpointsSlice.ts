import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { RestoreCheckpointsResponse } from "./types";

export type CheckpointsMeta = {
  latestCheckpointResult: RestoreCheckpointsResponse;
  isVisible: boolean;
  isUndoing: boolean;
};

const initialState: CheckpointsMeta = {
  latestCheckpointResult: {
    reverted_to: "",
    checkpoints_for_undo: [],
    reverted_changes: [],
    error_log: [],
  },
  isVisible: false,
  isUndoing: false,
};

export const checkpointsSlice = createSlice({
  name: "checkpoints",
  initialState,
  reducers: {
    setLatestCheckpointResult: (
      state,
      action: PayloadAction<RestoreCheckpointsResponse>,
    ) => {
      state.latestCheckpointResult = action.payload;
    },
    setIsCheckpointsPopupIsVisible: (state, action: PayloadAction<boolean>) => {
      state.isVisible = action.payload;
    },
    setIsUndoingCheckpoints: (state, action: PayloadAction<boolean>) => {
      state.isUndoing = action.payload;
    },
  },

  selectors: {
    selectLatestCheckpointResult: (state) => state.latestCheckpointResult,
    selectIsCheckpointsPopupIsVisible: (state) => state.isVisible,
    selectIsUndoingCheckpoints: (state) => state.isUndoing,
  },
});

export const {
  setLatestCheckpointResult,
  setIsCheckpointsPopupIsVisible,
  setIsUndoingCheckpoints,
} = checkpointsSlice.actions;
export const {
  selectLatestCheckpointResult,
  selectIsCheckpointsPopupIsVisible,
  selectIsUndoingCheckpoints,
} = checkpointsSlice.selectors;
