import React, {
  forwardRef,
  useImperativeHandle,
  useCallback,
  useRef,
  useEffect,
} from "react";
import { Box, type BoxProps } from "@radix-ui/themes";
import {
  useScrollContext,
  scrollAreaWithAnchorReducer,
  ScrollAreaWithAnchorContext,
} from "./useScrollContext";
import {
  ScrollArea as BaseScrollArea,
  type ScrollAreaProps,
} from "./ScrollArea";
import { useSpaceCalculator } from "./useSapceCalculator";
import { FollowButton } from "./FollowButton";
import { useIsIntersecting } from "./useIsIntersecting";

/**
 * Check list
 * Static chat
 * ✅ When give a long chat it should start from the last user message
 * ✅ When clicking the follow button it should go to the bottom of the screen
 * ✅ When at the bottom the follow button should not show
 *
 * In progress chat.
 * ✅ When a user message is submitted it should go to the user message
 * ✅ When i click the follow button it should follow the chat
 *
 */
export const ScrollArea = forwardRef<HTMLDivElement, ScrollAreaProps>(
  ({ children, ...props }, ref) => {
    return (
      <Provider {...props} ref={ref}>
        {children}
      </Provider>
    );
  },
);
ScrollArea.displayName = "ScrollAreaWithAnchor";

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
    scrollRef: scrollRef,
    innerRef: innerRef,
    bottomRef: null,
    anchorRef: null,
    anchorProps: null,
    scrolled: false,
    mode: "user-message",
  });

  const handleScroll = useCallback(
    (event: React.UIEvent<HTMLDivElement>) => {
      props.onScroll?.(event);
    },
    [props],
  );

  const handleWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      if (event.deltaY < 0) {
        dispatch({ type: "set_mode", payload: "manual" });
        dispatch({ type: "set_scrolled", payload: true });
      }
      props.onWheel?.(event);
    },
    [props],
  );

  return (
    <ScrollAreaWithAnchorContext.Provider value={{ state, dispatch }}>
      <BaseScrollArea
        ref={scrollRef}
        {...props}
        onWheel={handleWheel}
        onScroll={handleScroll}
      >
        <Box ref={innerRef}>
          {children}
          <BottomSpace />
        </Box>
        <FollowButton />
      </BaseScrollArea>
    </ScrollAreaWithAnchorContext.Provider>
  );
});
Provider.displayName = "ScrollAreaWithAnchor.Provider";

const BottomSpace: React.FC<BoxProps> = (props) => {
  const bottomRef = useRef<HTMLDivElement>(null);
  const { state, dispatch } = useScrollContext();
  const height = useSpaceCalculator(
    state.scrollRef?.current,
    state.innerRef?.current,
    state.anchorRef?.current,
    bottomRef.current,
  );

  useEffect(() => {
    dispatch({ type: "set_bottom", payload: bottomRef });
  }, [dispatch]);

  const nearBottom = useIsIntersecting(bottomRef.current, {
    threshold: 0.99,
    root: state.scrollRef?.current,
  });

  useEffect(() => {
    if (
      state.scrollRef?.current &&
      state.anchorRef?.current &&
      state.bottomRef?.current &&
      state.mode === "user-message" &&
      height &&
      !nearBottom
    ) {
      state.anchorRef.current.scrollIntoView(state.anchorProps ?? undefined);
      dispatch({ type: "set_scrolled", payload: true });
    }
  }, [
    state.scrollRef,
    height,
    state.anchorRef,
    state.anchorProps,
    state.bottomRef,
    state.mode,
    state.innerRef,
    dispatch,
    nearBottom,
    state.scrolled,
  ]);

  return <Box {...props} height={height + "px"} ref={bottomRef} />;
};

export type ScrollAnchorProps = React.PropsWithChildren<
  ScrollIntoViewOptions & BoxProps
>;

export const ScrollAnchor: React.FC<ScrollAnchorProps> = ({
  behavior,
  block,
  inline,
  ...props
}) => {
  const anchorRef = useRef<HTMLDivElement>(null);
  const { state, dispatch } = useScrollContext();

  useEffect(() => {
    dispatch({ type: "set_anchor", payload: anchorRef });
    dispatch({ type: "set_mode", payload: "user-message" });
    dispatch({
      type: "set_anchor_props",
      payload: { behavior, block, inline },
    });
    dispatch({ type: "set_scrolled", payload: false });
  }, [dispatch, behavior, block, inline]);

  useEffect(() => {
    if (state.mode !== "user-message" || state.scrolled) return;
    anchorRef.current?.scrollIntoView({ behavior, block, inline });
  }, [behavior, block, dispatch, inline, state.mode, state.scrolled]);

  return <Box {...props} ref={anchorRef} />;
};
