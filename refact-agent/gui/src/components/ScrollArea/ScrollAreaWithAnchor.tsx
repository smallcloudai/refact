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
import { useResizeObserver } from "../../hooks";
import { ScrollToBottomButton } from "./ScrollToBottomButton";
type State = {
  innerRef: RefObject<HTMLDivElement> | null;
  scrollRef: RefObject<HTMLDivElement> | null;
  anchorRef: RefObject<HTMLDivElement> | null;
  follow: boolean;
  anchorProps: ScrollIntoViewOptions | null;
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
  | { type: "set_follow"; payload: boolean }
  | { type: "set_anchor_props"; payload: ScrollIntoViewOptions | null }
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

    case "set_follow": {
      return {
        ...state,
        follow: action.payload,
      };
    }

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

    case "set_anchor_props": {
      return {
        ...state,
        anchor_props: action.payload,
      };
    }
    default:
      return state;
  }
}

function scrollToBottom(elem: HTMLElement) {
  elem.scrollTop = elem.scrollHeight - elem.clientHeight;
}

function calculateSpace(
  scrollElem: HTMLElement,
  anchorElem: HTMLElement,
  bottomElem: HTMLElement,
) {
  const anchorPosition = anchorElem.offsetTop;
  const topOfBottom = bottomElem.offsetTop;
  const spaceBetween = topOfBottom - anchorPosition;
  const maxSpace = scrollElem.clientHeight;
  return Math.max(maxSpace - spaceBetween, 0);
}

function useSpaceCalculator(
  scrollElem: HTMLElement | null,
  innerElem: HTMLElement | null,
  anchorElem: HTMLElement | null,
  bottomElem: HTMLElement | null,
) {
  const [height, setHeight] = useState<number>(0);
  const calculateAndSetSpace = useCallback(() => {
    if (!scrollElem || !bottomElem || !anchorElem) {
      return;
    }
    const nextHeight = calculateSpace(scrollElem, anchorElem, bottomElem);
    setHeight(nextHeight);
  }, [scrollElem, bottomElem, anchorElem]);

  useResizeObserver(innerElem, calculateAndSetSpace);
  useEffect(() => {
    calculateAndSetSpace();
  }, [calculateAndSetSpace]);

  return height;
}

function useFollowBottom(
  follow: boolean,
  scrollElem: HTMLElement | null,
  bottomElem: HTMLElement | null,
) {
  const [isIntersecting, setIsIntersecting] = useState(!follow);
  const followFn: IntersectionObserverCallback = useCallback(
    (entries) => {
      if (!scrollElem || !bottomElem) return;
      const btm = entries.find((e) => e.target === bottomElem);
      if (btm) {
        setIsIntersecting(btm.isIntersecting);
      }
      if (follow && btm && !btm.isIntersecting) {
        scrollToBottom(scrollElem);
      }
    },
    [follow, scrollElem, bottomElem],
  );

  useEffect(() => {
    const observer = new IntersectionObserver(followFn, {
      root: scrollElem,
      threshold: 0.1,
    });

    if (bottomElem) {
      observer.observe(bottomElem);
    }

    return () => {
      if (bottomElem) {
        observer.unobserve(bottomElem);
      }
    };
  });

  const showButton = useMemo(
    () => !follow && !isIntersecting,
    [follow, isIntersecting],
  );

  return { showFollow: showButton };
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
  const bottomRef = React.useRef<HTMLDivElement>(null);
  const [state, dispatch] = React.useReducer(scrollAreaWithAnchorReducer, {
    // mutable dom refs
    scrollRef: scrollRef,
    innerRef: innerRef,
    anchorRef: null,
    follow: false,
    anchorProps: null,
    scrolled: false,
  });

  const handleScroll = useCallback(
    (event: React.UIEvent<HTMLDivElement>) => {
      dispatch({ type: "set_scrolled", payload: true });
      props.onScroll?.(event);
    },
    [props],
  );

  const handleWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      if (state.follow && event.deltaY < 0) {
        dispatch({ type: "set_follow", payload: false });
        dispatch({ type: "set_scrolled", payload: true });
      }
      props.onWheel?.(event);
    },
    [props, state.follow],
  );

  const { showFollow } = useFollowBottom(
    state.follow,
    scrollRef.current,
    bottomRef.current,
  );

  const bottomSpaceHeight = useSpaceCalculator(
    scrollRef.current,
    innerRef.current,
    state.anchorRef?.current ?? null,
    bottomRef.current,
  );

  useEffect(() => {
    if (state.anchorRef?.current) {
      const anchorPosition =
        state.anchorRef.current.getBoundingClientRect().top;
      if (anchorPosition > 0 && !state.scrolled) {
        state.anchorRef.current.scrollIntoView(state.anchorProps ?? undefined);
      }
    }
  }, [bottomSpaceHeight, state.anchorProps, state.anchorRef, state.scrolled]);

  const handleFollowButtonClick = useCallback(() => {
    if (state.scrollRef?.current) {
      scrollToBottom(state.scrollRef.current);
    }
    dispatch({ type: "set_follow", payload: true });
  }, [state.scrollRef]);

  return (
    <ScrollAreaWithAnchorContext.Provider value={{ state, dispatch }}>
      <BaseScrollArea
        ref={scrollRef}
        {...props}
        onWheel={handleWheel}
        onScroll={(e) => handleScroll}
      >
        <Box ref={innerRef}>
          {children}
          <BottomSpace height={bottomSpaceHeight} ref={bottomRef} />
        </Box>
        {showFollow && (
          <ScrollToBottomButton onClick={handleFollowButtonClick} />
        )}
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
      </Provider>
    );
  },
);
ScrollArea.displayName = "ScrollAreaWithAnchor.ScrollArea";

type BottomSpaceProps = Omit<BoxProps, "height"> & { height: number };
const BottomSpace = forwardRef<HTMLDivElement, BottomSpaceProps>(
  ({ height, ...props }, ref) => {
    return <Box ref={ref} {...props} height={height + "px"} />;
  },
);
BottomSpace.displayName = "ScrollAreaWithAnchor.BottomSpace";

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
  const { dispatch } = useScrollContext();

  useEffect(() => {
    dispatch({ type: "add_anchor", payload: anchorRef });
    dispatch({
      type: "set_anchor_props",
      payload: { behavior, block, inline },
    });

    return () => {
      dispatch({
        type: "set_anchor_props",
        payload: null,
      });
      dispatch({ type: "set_scrolled", payload: false });
    };
  }, [dispatch, anchorRef, behavior, block, inline]);

  return <Box {...props} ref={anchorRef} />;
};

export { ScrollArea, ScrollAnchor };

/**
 * Check list
 * Static chat
 * ✅ When give a long chat it should start from the last user message
 * ✅ When clicking the follow button it should go to the bottom of the screen
 * ✅When at the bottom the follow button should not show
 *
 * In progress chat.
 * ✅ When a user message is submitted it should go to the user message
 * ✅ When i click the follow button it should follow the chat
 * ✅ I can stop following by manually scrolling
 * ✅ follow button shouldn't flicker
 * ✅ ui should scroll flicker
 */
