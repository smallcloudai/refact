import React, {
  createContext,
  forwardRef,
  useCallback,
  useContext,
  useEffect,
  useImperativeHandle,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type RefObject,
} from "react";
import { Box, BoxProps } from "@radix-ui/themes";
import {
  ScrollArea as BaseScrollArea,
  type ScrollAreaProps,
} from "./ScrollArea";
import { useResizeObserverOnRef } from "../../hooks";
import { ScrollToBottomButton } from "./ScrollToBottomButton";
type State = {
  innerRef: RefObject<HTMLDivElement> | null;
  scrollRef: RefObject<HTMLDivElement> | null;
  anchorRef: RefObject<HTMLDivElement> | null;
  scroll: boolean;
  scrolled: boolean;
  follow: boolean;
  overflow: boolean;
  at_the_bottom: boolean;
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
  | { type: "set_scrolled"; payload: boolean }
  | { type: "set_follow"; payload: boolean }
  | { type: "set_overflow"; payload: boolean }
  | { type: "set_at_the_bottom"; payload: boolean };

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

    case "set_follow": {
      return {
        ...state,
        follow: action.payload,
      };
    }

    case "set_overflow": {
      return {
        ...state,
        overflow: action.payload,
      };
    }

    case "set_at_the_bottom": {
      return {
        ...state,
        at_the_bottom: action.payload,
      };
    }

    default:
      return state;
  }
}

function isAtBottom(element: HTMLDivElement | null) {
  if (element === null) return true;
  const { scrollHeight, scrollTop, clientHeight } = element;
  return Math.abs(scrollHeight - clientHeight - scrollTop) <= 1;
}

function isOverflowing(element: HTMLDivElement | null) {
  if (element === null) return false;
  const { scrollHeight, clientHeight } = element;
  return scrollHeight > clientHeight;
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
    follow: false,
    overflow: false,
    at_the_bottom: false,
  });

  const handleScroll = useCallback(
    (event: React.UIEvent<HTMLDivElement>) => {
      if (state.scrollRef?.current) {
        const atTheBottom = isAtBottom(state.scrollRef.current);
        dispatch({ type: "set_at_the_bottom", payload: atTheBottom });
      }
      props.onScroll?.(event);
    },
    [props, state.scrollRef],
  );

  const handleWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      if (state.follow && event.deltaY < 0) {
        dispatch({ type: "set_follow", payload: false });
      }
      props.onWheel?.(event);
    },
    [props, state.follow],
  );

  return (
    <ScrollAreaWithAnchorContext.Provider value={{ state, dispatch }}>
      <BaseScrollArea
        ref={scrollRef}
        {...props}
        onScroll={handleScroll}
        onWheel={handleWheel}
      >
        <Box ref={innerRef}>{children}</Box>
        <FollowButton />
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

  // TODO: extract to a function
  const calculateAndSetSpace = useCallback(() => {
    if (
      !state.scrollRef?.current ||
      !state.innerRef?.current ||
      !state.anchorRef?.current ||
      !bottomSpaceRef.current
    ) {
      return;
    }

    const anchorPosition = state.anchorRef.current.offsetTop;
    const topOfBottom = bottomSpaceRef.current.offsetTop;
    const spaceBetween = topOfBottom - anchorPosition;
    const maxSpace = state.scrollRef.current.clientHeight;
    setHeight(Math.max(maxSpace - spaceBetween, 0));

    if (!state.scrolled) {
      dispatch({ type: "set_scroll", payload: true });
    }
  }, [
    dispatch,
    state.anchorRef,
    state.innerRef,
    state.scrollRef,
    state.scrolled,
  ]);

  useResizeObserverOnRef(state.innerRef, calculateAndSetSpace);

  useEffect(() => {
    calculateAndSetSpace();
  }, [calculateAndSetSpace, dispatch]);

  // move this to resize observer
  useLayoutEffect(() => {
    if (!state.scrollRef?.current) return;
    const atTheBottom = isAtBottom(state.scrollRef.current);
    dispatch({ type: "set_at_the_bottom", payload: atTheBottom });
  }, [state.scrollRef, dispatch, height]);

  useEffect(() => {
    if (!state.scrollRef?.current) return;
    const overflowing = isOverflowing(state.scrollRef.current);
    dispatch({ type: "set_overflow", payload: overflowing });
  }, [dispatch, height, state.scrollRef]);

  return <Box ref={bottomSpaceRef} height={height + "px"} mt="-2" />;
};

export type ScrollAnchorProps = React.PropsWithChildren<
  ScrollIntoViewOptions & BoxProps
>;
const ScrollAnchor: React.FC<ScrollAnchorProps> = ({
  behavior,
  block,
  inline,
  ...props
}) => {
  const anchorRef = useRef<HTMLDivElement>(null);
  const { state, dispatch } = useScrollContext();

  useEffect(() => {
    dispatch({ type: "add_anchor", payload: anchorRef });
    // dispatch({ type: "set_scrolled", payload: false });
  }, [dispatch, anchorRef]);

  useEffect(() => {
    return () => {
      dispatch({ type: "set_scrolled", payload: false });
      dispatch({ type: "set_scroll", payload: false });
    };
  }, [dispatch]);

  useEffect(() => {
    if (
      !state.scrollRef?.current ||
      !state.anchorRef?.current ||
      state.scrolled ||
      !state.scroll
    ) {
      return;
    }

    state.anchorRef.current.scrollIntoView({ behavior, block, inline });

    dispatch({ type: "set_scrolled", payload: true });
  }, [
    state.anchorRef,
    state.scroll,
    dispatch,
    state.scrolled,
    behavior,
    block,
    inline,
    state.scrollRef,
  ]);

  return <Box {...props} ref={anchorRef} />;
};

const FollowButton: React.FC = () => {
  const { state, dispatch } = useScrollContext();
  console.log(state);

  const handleClick = useCallback(() => {
    dispatch({ type: "set_follow", payload: true });
  }, [dispatch]);

  const shouldShow = !state.at_the_bottom;

  if (!shouldShow) return false;
  return <ScrollToBottomButton onClick={handleClick} />;
};

export { ScrollArea, ScrollAnchor };
