import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { smallCloudApi } from "../../services/smallcloud";
import { threadMessagesSlice } from "../ThreadMessages";
import { isUsage } from "../../services/refact/chat";

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
      if (state.message) return state;
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
    // TODO: update ballance
    builder.addMatcher(
      threadMessagesSlice.actions.receiveThreadMessages.match,
      (state, action) => {
        if (
          !isUsage(action.payload.news_payload_thread_message.ftm_usage) ||
          state.dismissed ||
          state.message
        ) {
          return state;
        }

        if (
          action.payload.news_payload_thread_message.ftm_usage.coins <= 2000
        ) {
          state.type = "balance";
          state.message =
            "Your account is running low on credits. Please top up your account to continue using the service.";
        } else {
          state.dismissed = false;
        }

        return state;
      },
    );

    builder.addMatcher(
      smallCloudApi.endpoints.getUser.matchFulfilled,
      (state, action) => {
        if (state.dismissed) return state;
        if (state.message) return state;
        if (action.payload.metering_balance <= 2000) {
          state.type = "balance";
          state.message =
            "Your account is running low on credits. Please top up your account to continue using the service.";
        }
        return state;
      },
    );
  },
});

export const { setInformation, clearInformation, dismissBalanceLowCallout } =
  informationSlice.actions;
export const { getInformationMessage, showBalanceLowCallout } =
  informationSlice.selectors;
