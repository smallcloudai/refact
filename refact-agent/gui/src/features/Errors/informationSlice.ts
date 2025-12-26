import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { smallCloudApi } from "../../services/smallcloud";
import { applyChatEvent } from "../Chat/Thread/actions";

export type InformationSliceState = {
  message: string | null;
  type: "balance" | null;
  dismissed: boolean;
};

const initialState: InformationSliceState = {
  message: null,
  type: null,
  dismissed: false,
};
export const informationSlice = createSlice({
  name: "information",
  initialState,
  reducers: {
    setInformation: (state, action: PayloadAction<string>) => {
      if (state.message) return;
      state.message = action.payload;
    },
    clearInformation: (state, _action: PayloadAction) => {
      state.message = null;
    },

    dismissBalanceLowCallout: (state, _action: PayloadAction) => {
      state.dismissed = true;
      state.type = null;
      state.message = null;
    },
  },
  selectors: {
    getInformationMessage: (state) => state.message,
    getInformationType: (state) => state.type,
    getInformationDismissed: (state) => state.dismissed,
    showBalanceLowCallout: (state) =>
      state.type === "balance" && !state.dismissed,
  },

  extraReducers: (builder) => {
    // Listen to SSE events for metering balance updates (addCase must come before addMatcher)
    builder.addCase(applyChatEvent, (state, action) => {
      const event = action.payload;
      // Check for metering_balance in SSE events
      if ("metering_balance" in event && typeof event.metering_balance === "number") {
        const balance = event.metering_balance;
        if (state.dismissed && balance > 2000) {
          state.dismissed = false;
        }
        if (state.dismissed) return;
        if (state.message) return;
        if (balance <= 2000) {
          state.type = "balance";
          state.message =
            "Your account is running low on credits. Please top up your account to continue using the service.";
        }
      }
    });

    builder.addMatcher(
      smallCloudApi.endpoints.getUser.matchFulfilled,
      (state, action) => {
        if (state.dismissed) return;
        if (state.message) return;
        if (action.payload.metering_balance <= 2000) {
          state.type = "balance";
          state.message =
            "Your account is running low on credits. Please top up your account to continue using the service.";
        }
      },
    );
  },
});

export const { setInformation, clearInformation, dismissBalanceLowCallout } =
  informationSlice.actions;
export const { getInformationMessage, showBalanceLowCallout } =
  informationSlice.selectors;
