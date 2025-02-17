import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { Checkpoint, PreviewCheckpointsResponse } from "./types";

export type CheckpointsMeta = {
  latestCheckpointResult: PreviewCheckpointsResponse & {
    current_checkpoints: Checkpoint[];
  };
  isVisible: boolean;
  isUndoing: boolean;
  restoringUserMessageIndex: number | null;
  shouldNewChatBeStarted: boolean;
};

const initialState: CheckpointsMeta = {
  latestCheckpointResult: {
    reverted_to: "",
    checkpoints_for_undo: [],
    current_checkpoints: [],
    reverted_changes: [],
    error_log: [],
  },
  isVisible: false,
  isUndoing: false,
  restoringUserMessageIndex: null,
  shouldNewChatBeStarted: false,
};

export const checkpointsSlice = createSlice({
  name: "checkpoints",
  initialState,
  reducers: {
    setLatestCheckpointResult: (
      state,
      action: PayloadAction<
        PreviewCheckpointsResponse & {
          messageIndex: number;
          current_checkpoints: Checkpoint[];
        }
      >,
    ) => {
      state.latestCheckpointResult = action.payload;
      state.restoringUserMessageIndex = action.payload.messageIndex;
    },
    setIsCheckpointsPopupIsVisible: (state, action: PayloadAction<boolean>) => {
      state.isVisible = action.payload;
    },
    setIsUndoingCheckpoints: (state, action: PayloadAction<boolean>) => {
      state.isUndoing = action.payload;
    },
    setShouldNewChatBeStarted: (state, action: PayloadAction<boolean>) => {
      state.shouldNewChatBeStarted = action.payload;
    },
    setCheckpointsErrorLog: (state, action: PayloadAction<string[]>) => {
      state.latestCheckpointResult.error_log = action.payload;
    },
    clearCheckpointsErrorLog: (state) => {
      state.latestCheckpointResult.error_log = [];
    },
  },

  selectors: {
    selectLatestCheckpointResult: (state) => state.latestCheckpointResult,
    selectIsCheckpointsPopupIsVisible: (state) => state.isVisible,
    selectIsUndoingCheckpoints: (state) => state.isUndoing,
    selectShouldNewChatBeStarted: (state) => state.shouldNewChatBeStarted,
    selectCheckpointsMessageIndex: (state) => state.restoringUserMessageIndex,
  },
});

export const {
  setLatestCheckpointResult,
  setIsCheckpointsPopupIsVisible,
  setIsUndoingCheckpoints,
  setShouldNewChatBeStarted,
  setCheckpointsErrorLog,
  clearCheckpointsErrorLog,
} = checkpointsSlice.actions;
export const {
  selectLatestCheckpointResult,
  selectIsCheckpointsPopupIsVisible,
  selectIsUndoingCheckpoints,
  selectShouldNewChatBeStarted,
  selectCheckpointsMessageIndex,
} = checkpointsSlice.selectors;
