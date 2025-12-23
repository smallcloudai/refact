import { createReducer, createAction } from "@reduxjs/toolkit";
import { RootState } from "../../app/store";

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

export const selectThreadProjectOrCurrentProject = (state: RootState) => {
  if (state.chat.thread.integration?.project) {
    return state.chat.thread.integration.project;
  }
  return state.chat.thread.project_name ?? state.current_project.name;
};
