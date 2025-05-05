import { createSlice } from "@reduxjs/toolkit";
import { smallCloudApi } from "../../services/smallcloud";
import { chatResponse } from "../Chat";
import { isChatResponseChoice } from "../../events";

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
    ),
      builder.addMatcher(chatResponse.match, (state, action) => {
        if (!isChatResponseChoice(action.payload)) return state;
        if (
          "metering_balance" in action.payload &&
          typeof action.payload.metering_balance === "number"
        ) {
          state.balance = action.payload.metering_balance;
        }
      });
  },

  selectors: {
    selectBalance: (state) => state.balance,
  },
});

export const { selectBalance } = coinBallanceSlice.selectors;
