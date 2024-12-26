import { createSlice, PayloadAction } from "@reduxjs/toolkit";

export type PauseReason = {
  type: "confirmation" | "denial";
  command: string;
  rule: string;
  tool_call_id: string;
};

export type ConfirmationState = {
  pauseReasons: PauseReason[];
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
    setPauseReasons(state, action: PayloadAction<PauseReason[]>) {
      state.pause = true;
      state.pauseReasons = action.payload;
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

export const { setPauseReasons, clearPauseReasonsAndHandleToolsStatus } =
  confirmationSlice.actions;
export const {
  getPauseReasonsWithPauseStatus,
  getToolsConfirmationStatus,
  getToolsInteractionStatus,
  getConfirmationPauseStatus,
} = confirmationSlice.selectors;
