import { useImperativeHandle, useEffect, useRef, useState } from "react";
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
    };
  }, []);

  const handleScroll: React.UIEventHandler<HTMLDivElement> = (event) => {
    if (!innerRef.current) return;
    const parent = event.currentTarget.getBoundingClientRect();
    const { bottom, height, top } = innerRef.current.getBoundingClientRect();
    const nextIsVisable =
      top <= parent.top
        ? parent.top - top <= height + 20
        : bottom - parent.bottom <= height + 20;

    setAutoScroll(nextIsVisable);
  };

  return { handleScroll, innerRef };
}
