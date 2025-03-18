import { useEffect } from "react";

export function useResizeObserverOnRef(
  ref: React.RefObject<HTMLElement> | null,
  callback: ResizeObserverCallback,
  options?: ResizeObserverOptions,
) {
  useEffect(() => {
    const observer = new ResizeObserver(callback);
    ref?.current && observer.observe(ref.current, options);
    return () => observer.disconnect();
  }, [callback, options, ref]);
}

export function useResizeObserver(
  elem: HTMLElement | null,
  callback: ResizeObserverCallback,
) {
  useEffect(() => {
    const observer = new ResizeObserver(callback);
    elem && observer.observe(elem);
    return () => observer.disconnect();
  }, [callback, elem]);
}
