import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { ToolConfirmationPauseReason } from "../../events";

export type ConfirmationState = {
  pauseReasons: ToolConfirmationPauseReason[];
  pause: boolean;
  status: {
    wasInteracted: boolean;
    confirmationStatus: boolean;
  };
};

const initialState: ConfirmationState = {
  pauseReasons: [],
  pause: false,
  status: {
    wasInteracted: false,
    confirmationStatus: true,
  },
};

type ConfirmationActionPayload = {
  wasInteracted: boolean;
  confirmationStatus: boolean;
};

export const confirmationSlice = createSlice({
  name: "confirmation",
  initialState,
  reducers: {
    setPauseReasons(
      state,
      action: PayloadAction<ToolConfirmationPauseReason[]>,
    ) {
      state.pause = true;
      state.pauseReasons = action.payload;
    },
    resetConfirmationInteractedState(state) {
      state.status.wasInteracted = false;
      state.pause = false;
      state.pauseReasons = [];
    },
    clearPauseReasonsAndHandleToolsStatus(
      state,
      action: PayloadAction<ConfirmationActionPayload>,
    ) {
      state.pause = false;
      state.pauseReasons = [];
      state.status = action.payload;
    },
  },
  selectors: {
    getPauseReasonsWithPauseStatus: (state) => state,
    getToolsInteractionStatus: (state) => state.status.wasInteracted,
    getToolsConfirmationStatus: (state) => state.status.confirmationStatus,
    getConfirmationPauseStatus: (state) => state.pause,
  },
});

export const {
  setPauseReasons,
  resetConfirmationInteractedState,
  clearPauseReasonsAndHandleToolsStatus,
} = confirmationSlice.actions;
export const {
  getPauseReasonsWithPauseStatus,
  getToolsConfirmationStatus,
  getToolsInteractionStatus,
  getConfirmationPauseStatus,
} = confirmationSlice.selectors;
