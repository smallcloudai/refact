import { RefObject, useCallback } from "react";

export function useHideScroll(ref: RefObject<HTMLElement>) {
  const hideScroll = useCallback(() => {
    ref.current?.scrollIntoView({ block: "nearest" });
  }, [ref]);

  return hideScroll;
}
