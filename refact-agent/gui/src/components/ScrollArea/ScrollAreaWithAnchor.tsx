import React, {
  createContext,
  forwardRef,
  useContext,
  useEffect,
  useImperativeHandle,
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
  scrolled: boolean;
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
    }
  | { type: "set_scrolled"; payload: boolean };

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

    case "set_scrolled": {
      return {
        ...state,
        scrolled: action.payload,
      };
    }

    default:
      return state;
  }
}

const Provider: React.FC<ScrollAreaProps> = forwardRef<
  HTMLDivElement,
  ScrollAreaProps
>(({ children, ...props }, ref) => {
  const scrollRef = React.useRef<HTMLDivElement>(null);
  useImperativeHandle<HTMLDivElement | null, HTMLDivElement | null>(
    ref,
    () => scrollRef.current,
  );
  const innerRef = React.useRef<HTMLDivElement>(null);
  const [state, dispatch] = React.useReducer(scrollAreaWithAnchorReducer, {
    // mutable dom refs
    scrollRef: scrollRef,
    innerRef: innerRef,
    anchorRef: null,
    scroll: false,
    scrolled: false,
  });

  return (
    <ScrollAreaWithAnchorContext.Provider value={{ state, dispatch }}>
      <BaseScrollArea ref={scrollRef} {...props}>
        <Box ref={innerRef}>{children}</Box>
      </BaseScrollArea>
    </ScrollAreaWithAnchorContext.Provider>
  );
});
Provider.displayName = "ScrollAreaWithAnchor.Provider";

function useScrollContext() {
  const context = useContext(ScrollAreaWithAnchorContext);
  if (context === null) {
    throw new Error("useScrollContext must be used within a CountProvider");
  }
  return context;
}

const ScrollArea = forwardRef<HTMLDivElement, ScrollAreaProps>(
  ({ children, ...props }, ref) => {
    return (
      <Provider {...props} ref={ref}>
        {children}
        <BottomSpace />
      </Provider>
    );
  },
);
ScrollArea.displayName = "ScrollAreaWithAnchor.ScrollArea";

const BottomSpace: React.FC = () => {
  const { state, dispatch } = useScrollContext();
  const [height, setHeight] = useState<number>(0);
  const bottomSpaceRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (
      !state.scrollRef?.current ||
      !state.innerRef?.current ||
      !state.anchorRef?.current ||
      state.scrolled
    ) {
      return;
    }

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

    const scrollViewportHeight = state.scrollRef.current.clientHeight;
    const anchorPosition = state.anchorRef.current.offsetTop;
    const contentHeight = state.innerRef.current.clientHeight;
    const distanceToBottom = contentHeight - anchorPosition;

    // If the distance from anchor to bottom is less than viewport height,
    // we need to add extra space to ensure the anchor can be properly scrolled to
    if (distanceToBottom < scrollViewportHeight) {
      const additionalSpace = scrollViewportHeight - distanceToBottom;
      setHeight(height + additionalSpace);
      dispatch({ type: "set_scroll", payload: true });
    } else {
      // There's already enough space, just enable scrolling
      dispatch({ type: "set_scroll", payload: true });
    }
  }, [
    state.scrollRef,
    state.innerRef,
    height,
    state.anchorRef,
    dispatch,
    state.scrolled,
  ]);

  //TODO: 8px extra space somewhere
  return <Box ref={bottomSpaceRef} style={{ height: height - 8 }} />;
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
    if (state.anchorRef?.current && state.scroll && !state.scrolled) {
      state.anchorRef.current.scrollIntoView(scrollTo);
      dispatch({ type: "set_scrolled", payload: true });
    }
  }, [state.anchorRef, scrollTo, state.scroll, dispatch, state.scrolled]);

  return (
    <Box ref={anchorRef} title="anchor">
      {children}
    </Box>
  );
};

export { ScrollArea, ScrollAnchor };
