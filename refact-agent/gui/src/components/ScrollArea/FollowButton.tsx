import { useCallback, useEffect, useMemo } from "react";
import { scrollToBottom } from "./utils";
import { useScrollContext } from "./useScrollContext";
import { ScrollToBottomButton } from "./ScrollToBottomButton";
import { useIsIntersecting } from "./useIsIntersecting";

export const FollowButton: React.FC = () => {
  const { state, dispatch } = useScrollContext();
  const isIntersecting = useIsIntersecting(state.bottomRef?.current ?? null, {
    threshold: 0.05,
    root: state.scrollRef?.current,
  });

  useEffect(() => {
    if (
      state.bottomRef?.current &&
      state.mode === "follow" &&
      !isIntersecting &&
      !state.scrolled
    ) {
      state.bottomRef.current.scrollIntoView({
        ...state.anchorProps,
        block: "end",
      });
    }
  });

  const handleFollowButtonClick = useCallback(() => {
    if (state.scrollRef?.current) {
      scrollToBottom(state.scrollRef.current);
    }
    dispatch({ type: "set_mode", payload: "follow" });
    dispatch({ type: "set_scrolled", payload: false });
  }, [dispatch, state.scrollRef]);

  const showButton = useMemo(
    () => state.mode !== "follow" && !isIntersecting,
    [state.mode, isIntersecting],
  );

  if (!showButton) return null;

  return <ScrollToBottomButton onClick={handleFollowButtonClick} />;
};
