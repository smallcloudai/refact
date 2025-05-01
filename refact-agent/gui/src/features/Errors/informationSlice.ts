import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import { chatResponse } from "../Chat";
import { smallCloudApi } from "../../services/smallcloud";

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
    builder.addMatcher(chatResponse.match, (state, action) => {
      if (
        state.dismissed &&
        "metering_balance" in action.payload &&
        typeof action.payload.metering_balance === "number" &&
        action.payload.metering_balance > 2000
      ) {
        state.dismissed = false;
      }
      if (state.dismissed) return state;
      if (state.message) return state;
      if (!("metering_balance" in action.payload)) return state;
      if (typeof action.payload.metering_balance !== "number") return state;
      if (action.payload.metering_balance <= 2000) {
        state.type = "balance";
        state.message =
          "Your account is running low on credits. Please top up your account to continue using the service.";
      }
      return state;
    });

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
