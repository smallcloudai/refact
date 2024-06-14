import { useImperativeHandle, useEffect, useRef, useState } from "react";
import { type ChatMessages } from "../../events";

type useAutoScrollProps = {
  ref: React.ForwardedRef<HTMLDivElement>;
  messages: ChatMessages;
};

export function useAutoScroll({ ref, messages }: useAutoScrollProps) {
  const innerRef = useRef<HTMLDivElement>(null);
  // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
  useImperativeHandle(ref, () => innerRef.current!, []);

  const [autoScroll, setAutoScroll] = useState(true);

  useEffect(() => {
    if (autoScroll && innerRef.current?.scrollIntoView) {
      innerRef.current.scrollIntoView({ behavior: "instant", block: "end" });
    }
  }, [messages, autoScroll]);

  const handleScroll: React.UIEventHandler<HTMLDivElement> = (event) => {
    if (!innerRef.current) return;
    const parent = event.currentTarget.getBoundingClientRect();
    const { bottom, height, top } = innerRef.current.getBoundingClientRect();
    const isVisable =
      top <= parent.top
        ? parent.top - top <= height
        : bottom - parent.bottom <= height;

    if (isVisable && !autoScroll) {
      setAutoScroll(true);
    } else if (autoScroll) {
      setAutoScroll(false);
    }
  };

  return { handleScroll, innerRef };
}
