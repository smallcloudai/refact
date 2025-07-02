import { createSlice } from "@reduxjs/toolkit";
import { smallCloudApi } from "../../services/smallcloud";
import { threadMessagesSlice } from "../ThreadMessages";
import { isUsage } from "../../services/refact";

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
    builder.addMatcher(
      smallCloudApi.endpoints.getUser.matchFulfilled,
      (state, action) => {
        state.balance = action.payload.metering_balance;
      },
    );
    builder.addMatcher(
      threadMessagesSlice.actions.receiveThreadMessages.match,
      (state, action) => {
        if (!isUsage(action.payload.news_payload_thread_message.ftm_usage))
          return state;

        state.balance =
          action.payload.news_payload_thread_message.ftm_usage.coins;
      },
    );
  },

  selectors: {
    selectBalance: (state) => state.balance,
  },
});

export const { selectBalance } = coinBallanceSlice.selectors;
