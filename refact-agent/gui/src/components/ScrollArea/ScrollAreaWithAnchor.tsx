import React, {
  createContext,
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from "react";
import { Box } from "@radix-ui/themes";
import {
  ScrollArea as BaseScrollArea,
  type ScrollAreaProps,
} from "./ScrollArea";
type State = {
  innerRef: RefObject<HTMLDivElement> | null;
  scrollRef: RefObject<HTMLDivElement> | null;
  anchorRef: RefObject<HTMLDivElement> | null;
  scroll: boolean;
};

type Action =
  | {
      type: "add_anchor";
      payload: RefObject<HTMLDivElement>;
    }
  | {
      type: "upsert_refs";
      payload: Partial<State>;
    }
  | {
      type: "set_scroll";
      payload: boolean;
    };

type Dispatch = (action: Action) => void;

const ScrollAreaWithAnchorContext = createContext<{
  state: State;
  dispatch: Dispatch;
} | null>(null);

function scrollAreaWithAnchorReducer(state: State, action: Action) {
  switch (action.type) {
    case "upsert_refs": {
      return {
        ...state,
        ...action.payload,
      };
    }
    case "add_anchor":
      return {
        ...state,
        anchorRef: action.payload,
      };

    case "set_scroll": {
      return {
        ...state,
        scroll: action.payload,
      };
    }

    default:
      return state;
  }
}

const Provider: React.FC<ScrollAreaProps> = ({ children, ...props }) => {
  const scrollRef = React.useRef<HTMLDivElement>(null);
  const innerRef = React.useRef<HTMLDivElement>(null);
  const [state, dispatch] = React.useReducer(scrollAreaWithAnchorReducer, {
    // mutable dom refs
    scrollRef: scrollRef,
    innerRef: innerRef,
    anchorRef: null,
    scroll: false,
  });

  return (
    <ScrollAreaWithAnchorContext.Provider value={{ state, dispatch }}>
      <BaseScrollArea ref={scrollRef} {...props}>
        <Box ref={innerRef}>{children}</Box>
      </BaseScrollArea>
    </ScrollAreaWithAnchorContext.Provider>
  );
};

function useScrollContext() {
  const context = useContext(ScrollAreaWithAnchorContext);
  if (context === null) {
    throw new Error("useScrollContext must be used within a CountProvider");
  }
  return context;
}

const ScrollArea: React.FC<ScrollAreaProps> = ({ children, ...props }) => {
  return (
    <Provider {...props}>
      {children}
      <BottomSpace />
    </Provider>
  );
};

const BottomSpace: React.FC = () => {
  const { state, dispatch } = useScrollContext();
  const [height, setHeight] = useState<number>(0);
  const bottomSpaceRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    // TODO: calculate space needed from anchor to bottom
    if (
      !state.scrollRef?.current ||
      !state.innerRef?.current ||
      !state.anchorRef?.current
    )
      return;

    // console.log(
    //   state.scrollRef.current.clientHeight,
    //   state.innerRef.current.clientHeight,
    //   height,
    // );

    // case viewport isn't big enough to scroll
    if (
      state.innerRef.current.clientHeight + height <
      state.scrollRef.current.clientHeight
    ) {
      const fillSize =
        state.scrollRef.current.clientHeight -
        state.innerRef.current.clientHeight;
      const anchorTop = Math.max(state.anchorRef.current.offsetTop, 0);
      setHeight(fillSize + anchorTop);
      dispatch({ type: "set_scroll", payload: true });
      return;
    }

    // case viewport is big enough to scroll, but there's not enough room at the end
    // anchor ref and this should be client height away from each other
  }, [state.scrollRef, state.innerRef, height, state.anchorRef, dispatch]);

  return <Box ref={bottomSpaceRef} style={{ height: height }} />;
};

export type ScrollAnchorProps = React.PropsWithChildren<ScrollIntoViewOptions>;
const ScrollAnchor: React.FC<ScrollAnchorProps> = ({
  children,
  ...scrollTo
}) => {
  // const [scrolled, setScrolled] = useState(false);
  const anchorRef = useRef<HTMLDivElement>(null);
  const { state, dispatch } = useScrollContext();

  useEffect(() => {
    dispatch({ type: "add_anchor", payload: anchorRef });
  }, [dispatch, anchorRef]);

  useLayoutEffect(() => {
    if (state.anchorRef?.current && state.scroll) {
      state.anchorRef.current.scrollIntoView(scrollTo);
      dispatch({ type: "set_scroll", payload: false });
    }
  }, [state.anchorRef, scrollTo, state.scroll, dispatch]);

  return (
    <Box ref={anchorRef} title="anchor">
      {children}
    </Box>
  );
};

export { ScrollArea, ScrollAnchor };
