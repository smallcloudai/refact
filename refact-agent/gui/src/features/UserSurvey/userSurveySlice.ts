import { createSlice } from "@reduxjs/toolkit";

const ONE_DAY_IN_MS = 1000 * 60 * 60 * 24;

type UserSurveySlice = {
  lastAsked: number;
};
const initialState: UserSurveySlice = {
  lastAsked: 0,
};
export const userSurveySlice = createSlice({
  name: "userSurvey",
  initialState,
  reducers: {
    setLastAsked: (state) => {
      const now = Date.now();
      state.lastAsked = now;
    },
  },

  selectors: {
    userSurveyWasAskedMoreThanADayAgo: (state) => {
      const now = Date.now();
      return now - state.lastAsked > ONE_DAY_IN_MS;
    },
  },
});

export const { setLastAsked } = userSurveySlice.actions;
export const { userSurveyWasAskedMoreThanADayAgo } = userSurveySlice.selectors;
