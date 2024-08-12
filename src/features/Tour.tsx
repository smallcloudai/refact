import { createAction, createReducer } from "@reduxjs/toolkit";
import { createContext, useContext, useState } from "react";

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

function isTourInProgress(state: unknown): state is TourInProgress {
  if (!state) return false;
  if (typeof state !== "object") return false;
  if (!("type" in state)) return false;
  if (state.type !== "in_progress") return false;
  if (!("step" in state)) return false;
  if (typeof state.step !== "number") return false;
  return true;
}

function isTourClosed(state: unknown): state is TourClosed {
  if (!state) return false;
  if (typeof state !== "object") return false;
  if (!("type" in state)) return false;
  if (state.type !== "closed") return false;
  if (!("step" in state)) return false;
  if (typeof state.step !== "number") return false;
  return true;
}

function isTourFinished(state: unknown): state is TourClosed {
  if (!state) return false;
  if (typeof state !== "object") return false;
  if (!("type" in state)) return false;
  if (state.type !== "finished") return false;
  return true;
}

function isTourState(state: unknown): state is TourState {
  return (
    isTourInProgress(state) || isTourClosed(state) || isTourFinished(state)
  );
}

export const initialState: TourState = {
  type: "in_progress",
  step: 1,
};

export const next = createAction("tour/next");
export const close = createAction("tour/close");

function loadFromLocalStorage(): TourState {
  try {
    const serialisedState = localStorage.getItem("tour");
    if (serialisedState === null) return initialState;
    const parsedState: unknown = JSON.parse(serialisedState);
    if (!isTourState(parsedState)) return initialState;
    return parsedState;
  } catch (e) {
    // eslint-disable-next-line no-console
    console.warn(e);
    return initialState;
  }
}

export const saveTourToLocalStorage = (state: { tour: TourState }) => {
  try {
    localStorage.setItem("tour", JSON.stringify(state.tour));
  } catch (e) {
    // eslint-disable-next-line no-console
    console.warn(e);
  }
};

export const tourReducer = createReducer<TourState>(
  loadFromLocalStorage(),
  (builder) => {
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
  },
);

export type TourRefs = {
  newChat: null | HTMLButtonElement;
  useTools: null | HTMLDivElement;
  useModel: null | HTMLDivElement;
  chat: null | HTMLDivElement;
  openInNewTab: null | HTMLButtonElement;
  newChatInside: null | HTMLButtonElement;
  back: null | HTMLButtonElement;
  f1: null | HTMLButtonElement;
  more: null | HTMLButtonElement;
  setNewChat: (x: HTMLButtonElement | null) => void;
  setUseTools: (x: HTMLDivElement | null) => void;
  setUseModel: (x: HTMLDivElement | null) => void;
  setChat: (x: HTMLDivElement | null) => void;
  setOpenInNewTab: (x: HTMLButtonElement | null) => void;
  setNewChatInside: (x: HTMLButtonElement | null) => void;
  setBack: (x: HTMLButtonElement | null) => void;
  setF1: (x: HTMLButtonElement | null) => void;
  setMore: (x: HTMLButtonElement | null) => void;
};

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const TourContext = createContext<TourRefs>(null!);

type TourContextProps = {
  children: React.ReactNode;
};

export const TourProvider = ({ children }: TourContextProps) => {
  const [newChat, setNewChat] = useState<null | HTMLButtonElement>(null);
  const [useTools, setUseTools] = useState<null | HTMLDivElement>(null);
  const [useModel, setUseModel] = useState<null | HTMLDivElement>(null);
  const [chat, setChat] = useState<null | HTMLDivElement>(null);
  const [openInNewTab, setOpenInNewTab] = useState<null | HTMLButtonElement>(
    null,
  );
  const [newChatInside, setNewChatInside] = useState<null | HTMLButtonElement>(
    null,
  );
  const [back, setBack] = useState<null | HTMLButtonElement>(null);
  const [f1, setF1] = useState<null | HTMLButtonElement>(null);
  const [more, setMore] = useState<null | HTMLButtonElement>(null);

  return (
    <TourContext.Provider
      value={{
        newChat,
        useTools,
        useModel,
        chat,
        openInNewTab,
        newChatInside,
        back,
        f1,
        more,
        setNewChat,
        setUseTools,
        setUseModel,
        setChat,
        setOpenInNewTab,
        setNewChatInside,
        setBack,
        setF1,
        setMore,
      }}
    >
      {children}
    </TourContext.Provider>
  );
};

export const useTourRefs = () => {
  const context = useContext(TourContext);
  return context;
};
