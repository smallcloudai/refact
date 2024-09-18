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

export type TourState = TourInProgress | TourClosed | TourFinished;

const initialState: TourState = {
  type: "in_progress",
  step: 1,
};

// eslint-disable-next-line react-refresh/only-export-components
export const next = createAction("tour/next");
// eslint-disable-next-line react-refresh/only-export-components
export const close = createAction("tour/close");
// eslint-disable-next-line react-refresh/only-export-components
export const finish = createAction("tour/finish");
// eslint-disable-next-line react-refresh/only-export-components
export const restart = createAction("tour/restart");

// eslint-disable-next-line react-refresh/only-export-components
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
  builder.addCase(finish, () => {
    return { type: "finished" };
  });
  builder.addCase(restart, () => {
    return { type: "in_progress", step: 1 };
  });
});

export type TourRefs = {
  newChat: null | HTMLButtonElement;
  useTools: null | HTMLDivElement;
  useModel: null | HTMLDivElement;
  chat: null | HTMLDivElement;
  openInNewTab: null | HTMLButtonElement;
  back: null | HTMLAnchorElement;
  f1: null | HTMLButtonElement;
  more: null | HTMLButtonElement;
  setNewChat: (x: HTMLButtonElement | null) => void;
  setUseTools: (x: HTMLDivElement | null) => void;
  setUseModel: (x: HTMLDivElement | null) => void;
  setChat: (x: HTMLDivElement | null) => void;
  setOpenInNewTab: (x: HTMLButtonElement | null) => void;
  setBack: (x: HTMLAnchorElement | null) => void;
  setF1: (x: HTMLButtonElement | null) => void;
  setMore: (x: HTMLButtonElement | null) => void;
};

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const TourContext = createContext<TourRefs>(null!);

type TourContextProps = {
  children: React.ReactNode;
};
// TODO: having a component here causes the linter warnings, Tour a directory, with separate files should for the component and actions fix this
export const TourProvider = ({ children }: TourContextProps) => {
  const [newChat, setNewChat] = useState<null | HTMLButtonElement>(null);
  const [useTools, setUseTools] = useState<null | HTMLDivElement>(null);
  const [useModel, setUseModel] = useState<null | HTMLDivElement>(null);
  const [chat, setChat] = useState<null | HTMLDivElement>(null);
  const [openInNewTab, setOpenInNewTab] = useState<null | HTMLButtonElement>(
    null,
  );
  const [back, setBack] = useState<null | HTMLAnchorElement>(null);
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
        back,
        f1,
        more,
        setNewChat,
        setUseTools,
        setUseModel,
        setChat,
        setOpenInNewTab,
        setBack,
        setF1,
        setMore,
      }}
    >
      {children}
    </TourContext.Provider>
  );
};

// eslint-disable-next-line react-refresh/only-export-components
export const useTourRefs = () => {
  const context = useContext(TourContext);
  return context;
};
