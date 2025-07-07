import { createReducer, createAction } from "@reduxjs/toolkit";

export type CurrentProjectInfo = {
  name: string;
};

const initialState: CurrentProjectInfo = {
  name: "",
};

export const setCurrentProjectInfo = createAction<CurrentProjectInfo>(
  "currentProjectInfo/setCurrentProjectInfo",
);

export const currentProjectInfoReducer = createReducer(
  initialState,
  (builder) => {
    builder.addCase(setCurrentProjectInfo, (_state, action) => {
      // state.name = action.payload.name;
      return action.payload;
    });
  },
);
