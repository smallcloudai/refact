import React, { useEffect, useState, useCallback } from "react";
import { useAppSelector } from "../../hooks";
import {
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
} from "../../features/Chat/Thread/selectors";

type useAutoScrollProps = {
  scrollRef: React.RefObject<HTMLDivElement>;
};

function isAtBottom(element: HTMLDivElement | null) {
  if (element === null) return true;
  const { scrollHeight, scrollTop, clientHeight } = element;
  return Math.abs(scrollHeight - (scrollTop + clientHeight)) <= 1;
}

function isOverflowing(element: HTMLDivElement | null) {
  if (element === null) return false;
  const { scrollHeight, clientHeight } = element;
  return scrollHeight > clientHeight;
}

export function useAutoScroll({ scrollRef }: useAutoScrollProps) {
  const [followRef, setFollowRef] = useState(true);

  const [isScrolledTillBottom, setIsScrolledTillBottom] = useState(true);

  const messages = useAppSelector(selectMessages);
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);

  const scrollIntoView = useCallback(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop =
        scrollRef.current.scrollHeight - scrollRef.current.clientHeight;
    }
  }, [scrollRef]);

  useEffect(() => {
    scrollIntoView();
  }, [scrollRef, scrollIntoView]);

  const handleScrollButtonClick = useCallback(() => {
    setFollowRef(isStreaming);
    scrollIntoView();
  }, [isStreaming, scrollIntoView]);

  // Check if at the bottom of the page.
  const handleScroll = useCallback(
    (_event: React.UIEvent<HTMLDivElement>) => {
      const bottom = isAtBottom(scrollRef.current);
      setIsScrolledTillBottom(bottom);
    },
    [scrollRef],
  );

  const handleWheel = useCallback(
    (event: React.WheelEvent<HTMLDivElement>) => {
      if (followRef && event.deltaY < 0) {
        setFollowRef(false);
      }
    },
    [followRef],
  );

  // Scroll to the end of the chat when the user clicks on the scroll button
  useEffect(() => {
    if (followRef) {
      scrollIntoView();
    }
  }, [followRef, scrollIntoView]);

  // Scroll when more messages come in
  useEffect(() => {
    if ((isWaiting || isStreaming) && followRef) {
      scrollIntoView();
    } else if ((isWaiting || isStreaming) && isOverflowing(scrollRef.current)) {
      const bottom = isAtBottom(scrollRef.current);
      setIsScrolledTillBottom(bottom);
    }
  }, [isStreaming, followRef, messages, scrollIntoView, isWaiting, scrollRef]);

  // reset on unmount
  useEffect(() => {
    return () => {
      setFollowRef(false);
      setIsScrolledTillBottom(false);
    };
  }, []);

  const showFollowButton =
    !followRef && isOverflowing(scrollRef.current) && !isScrolledTillBottom;

  return {
    handleScroll,
    handleWheel,
    handleScrollButtonClick,
    showFollowButton,
  };
}
