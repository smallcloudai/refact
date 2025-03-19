import { useState, useCallback, useEffect, useMemo } from "react";
import { scrollToBottom } from "./utils";
import { useScrollContext } from "./useScrollContext";
import { ScrollToBottomButton } from "./ScrollToBottomButton";

export const FollowButton: React.FC = () => {
  const { state, dispatch } = useScrollContext();
  const [isIntersecting, setIsIntersecting] = useState(state.follow);
  const followFn: IntersectionObserverCallback = useCallback(
    (entries) => {
      if (!state.scrollRef || !state.bottomRef) return;
      const btm = entries.find((e) => e.target === state.bottomRef?.current);

      if (btm) {
        setIsIntersecting(btm.isIntersecting);
      }
      if (
        state.scrollRef.current &&
        state.mode === "follow" &&
        btm &&
        !state.scrolled &&
        !btm.isIntersecting
      ) {
        scrollToBottom(state.scrollRef.current);
      }
    },
    [state.scrollRef, state.bottomRef, state.mode, state.scrolled],
  );
  // TODO: useIsIntersecting hook
  useEffect(() => {
    const observer = new IntersectionObserver(followFn, {
      root: state.scrollRef?.current,
      threshold: 0.2,
    });

    if (state.bottomRef?.current) {
      observer.observe(state.bottomRef.current);
    }

    return () => {
      observer.disconnect();
    };
  });

  const handleFollowButtonClick = useCallback(() => {
    if (state.scrollRef?.current) {
      scrollToBottom(state.scrollRef.current);
    }
    dispatch({ type: "set_mode", payload: "follow" });
    dispatch({ type: "set_scrolled", payload: false });
  }, [dispatch, state.scrollRef]);

  const showButton = useMemo(
    () => !state.follow && !isIntersecting,
    [state.follow, isIntersecting],
  );

  if (!showButton) return null;

  return <ScrollToBottomButton onClick={handleFollowButtonClick} />;
};
