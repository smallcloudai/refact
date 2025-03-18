import { createContext, useContext, type RefObject } from "react";
type State = {
  innerRef: RefObject<HTMLDivElement> | null;
  scrollRef: RefObject<HTMLDivElement> | null;
  anchorRef: RefObject<HTMLDivElement> | null;
  bottomRef: RefObject<HTMLDivElement> | null;
  follow: boolean;
  anchorProps: ScrollIntoViewOptions | null;
  scrolled: boolean;
};

type Action =
  | {
      type: "set_anchor";
      payload: RefObject<HTMLDivElement> | null;
    }
  | { type: "set_bottom"; payload: RefObject<HTMLDivElement> | null }
  | {
      type: "upsert_refs";
      payload: Partial<State>;
    }
  | { type: "set_follow"; payload: boolean }
  | { type: "set_anchor_props"; payload: ScrollIntoViewOptions | null }
  | { type: "set_scrolled"; payload: boolean };

type Dispatch = (action: Action) => void;

export const ScrollAreaWithAnchorContext = createContext<{
  state: State;
  dispatch: Dispatch;
} | null>(null);

export function useScrollContext() {
  const context = useContext(ScrollAreaWithAnchorContext);
  if (context === null) {
    throw new Error("useScrollContext must be used within a CountProvider");
  }
  return context;
}
export function scrollAreaWithAnchorReducer(state: State, action: Action) {
  switch (action.type) {
    case "upsert_refs": {
      return {
        ...state,
        ...action.payload,
      };
    }
    case "set_anchor":
      return {
        ...state,
        anchorRef: action.payload,
      };

    case "set_follow": {
      return {
        ...state,
        follow: action.payload,
      };
    }

    case "set_scrolled": {
      return {
        ...state,
        scrolled: action.payload,
      };
    }

    case "set_anchor_props": {
      return {
        ...state,
        anchor_props: action.payload,
      };
    }

    case "set_bottom": {
      return {
        ...state,
        bottomRef: action.payload,
      };
    }
    default:
      return state;
  }
}
