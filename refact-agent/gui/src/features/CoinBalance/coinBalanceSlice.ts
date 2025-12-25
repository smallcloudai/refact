import { createSlice } from "@reduxjs/toolkit";
import { smallCloudApi } from "../../services/smallcloud";
import { applyChatEvent } from "../Chat/Thread/actions";

type CoinBalance = {
  balance: number;
};
const initialState: CoinBalance = {
  balance: 0,
};
export const coinBallanceSlice = createSlice({
  name: "coins",
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    // Listen to SSE events for metering balance updates
    // Balance is now primarily updated via getUser query, but we can also
    // check for metering_balance in SSE events if the engine sends it
    builder.addCase(applyChatEvent, (state, action) => {
      const event = action.payload;
      // Check for metering_balance in runtime_updated or message events
      if ("metering_balance" in event && typeof event.metering_balance === "number") {
        state.balance = event.metering_balance;
      }
    });

    builder.addMatcher(
      smallCloudApi.endpoints.getUser.matchFulfilled,
      (state, action) => {
        state.balance = action.payload.metering_balance;
      },
    );
  },

  selectors: {
    selectBalance: (state) => state.balance,
  },
});

export const { selectBalance } = coinBallanceSlice.selectors;
