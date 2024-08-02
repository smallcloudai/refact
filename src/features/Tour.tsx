import { createAction, createReducer } from "@reduxjs/toolkit";

type TourInProgress = {
  type: "in_progress";
  step: number;
};

type TourClosed = {
  type: "closed";
  step: number;
};

type TourFinished = {
  type: "finished";
};

type TourState = TourInProgress | TourClosed | TourFinished;

export const initialState: TourState = {
  type: "in_progress",
  step: 1,
};

export const next = createAction("tour/next");
export const close = createAction("tour/close");

export const tourReducer = createReducer<TourState>(initialState, (builder) => {
  builder.addCase(next, (state) => {
    if (state.type === "in_progress") {
      return {
        ...state,
        step: state.step + 1,
      };
    }
    return state;
  });
  builder.addCase(close, (state) => {
    if (state.type === "in_progress") {
      return {
        ...state,
        type: "closed",
      };
    }
    return state;
  });
});
