import React, { useImperativeHandle, useEffect, useRef, useState } from "react";
import { type ChatMessages } from "../../services/refact";

type useAutoScrollProps = {
  ref: React.ForwardedRef<HTMLDivElement>;
  messages: ChatMessages;
  isStreaming: boolean;
};

export function useAutoScroll({
  ref,
  messages,
  isStreaming,
}: useAutoScrollProps) {
  const innerRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  useImperativeHandle(ref, () => innerRef.current!, []);

  const [autoScroll, setAutoScroll] = useState(true);
  const [lastScrollHeight, setLastScrollHeight] = useState(0);
  const [isScrolledTillBottom, setIsScrolledTillBottom] = useState(true);
  const [currentScrollHeight, setCurrentScrollHeight] = useState(0);

  /*
    Parent state is needed to calculate new scroll height of scroll area, if user clicks the button instead of scrolling down.
    Since the event is not giving us parent (click event is not giving a parent within `currentTarget`), 
    we need to save it to state to get updated scrollHeight later on
  */
  const [parent, setParent] = useState<(EventTarget & HTMLDivElement) | null>(
    null,
  );

  useEffect(() => {
    setAutoScroll(isStreaming);
  }, [isStreaming]);

  useEffect(() => {
    if (isStreaming && autoScroll && innerRef.current?.scrollIntoView) {
      innerRef.current.scrollIntoView({ behavior: "instant", block: "end" });
    }
  }, [messages, autoScroll, isStreaming]);

  useEffect(() => {
    return () => {
      setAutoScroll(true);
      setCurrentScrollHeight(0);
    };
  }, []);

  const handleScroll: React.UIEventHandler<HTMLDivElement> = (event) => {
    if (!innerRef.current) return;

    const currentTarget = event.currentTarget;
    const scrollHeight = parent?.scrollHeight ?? currentTarget.scrollHeight;

    if (!isStreaming) {
      setLastScrollHeight(scrollHeight);
    }

    if (lastScrollHeight < scrollHeight) {
      setCurrentScrollHeight(
        lastScrollHeight === 0
          ? lastScrollHeight
          : scrollHeight - lastScrollHeight,
      );
    } else {
      setLastScrollHeight(scrollHeight);
      setCurrentScrollHeight(0);
      setIsScrolledTillBottom(true);
      setAutoScroll(true);
    }

    setParent(currentTarget);

    const parentRect = currentTarget.getBoundingClientRect();
    const { bottom, height, top } = innerRef.current.getBoundingClientRect();

    const isBottomScrolled =
      top <= parentRect.top
        ? parentRect.top - top <= height + 20
        : bottom - parentRect.bottom <= height + 20;

    setIsScrolledTillBottom(isBottomScrolled);
    setAutoScroll(isBottomScrolled);
    if (currentScrollHeight > 630) {
      setAutoScroll(false);
      setIsScrolledTillBottom(false);
    }
  };

  const handleWheel: React.WheelEventHandler<HTMLDivElement> = (event) => {
    if (!isStreaming) return;

    if (event.deltaY < 0) {
      setAutoScroll(false);
    } else {
      setLastScrollHeight(event.currentTarget.scrollHeight);
      setAutoScroll(isScrolledTillBottom);
    }
  };

  const handleScrollButtonClick = () => {
    if (!innerRef.current || !parent) return;

    innerRef.current.scrollIntoView({ behavior: "instant", block: "end" });
    setAutoScroll(true);
    setIsScrolledTillBottom(true);
    setCurrentScrollHeight(0);
    setLastScrollHeight(parent.scrollHeight);
  };

  return {
    handleScroll,
    handleWheel,
    innerRef,
    isScrolledTillBottom,
    handleScrollButtonClick,
  };
}
