import { useImperativeHandle, useEffect, useRef, useState } from "react";
import { type ChatMessages } from "../../events";

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

  const [autoScroll, setAutoScroll] = useState(false);
  const [isVisable, setIsVisable] = useState(true);

  useEffect(() => {
    setAutoScroll(isStreaming);
  }, [isStreaming]);

  useEffect(() => {
    if (
      isStreaming &&
      !isVisable &&
      autoScroll &&
      innerRef.current?.scrollIntoView
    ) {
      innerRef.current.scrollIntoView({ behavior: "instant", block: "end" });
    }
  }, [messages, autoScroll, isVisable, isStreaming]);

  const handleScroll: React.UIEventHandler<HTMLDivElement> = (event) => {
    if (!innerRef.current) return;
    const parent = event.currentTarget.getBoundingClientRect();
    const { bottom, height, top } = innerRef.current.getBoundingClientRect();
    const isVisable =
      top <= parent.top
        ? parent.top - top <= height
        : bottom - parent.bottom <= height;
    setIsVisable(isVisable);
  };

  return { handleScroll, innerRef };
}
