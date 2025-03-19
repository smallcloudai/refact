import { useState, useCallback, useEffect } from "react";

export function useIsIntersecting(
  element: HTMLElement | null,
  options: IntersectionObserverInit,
) {
  const [isIntersecting, setIntersecting] = useState<boolean>(false);
  const callback: IntersectionObserverCallback = useCallback(
    (entries) => {
      const entry = entries.find((entry) => entry.target === element);
      setIntersecting(entry?.isIntersecting ?? false);
    },
    [element],
  );
  useEffect(() => {
    const observer = new IntersectionObserver(callback, options);
    if (element) observer.observe(element);
    return () => observer.disconnect();
  }, [callback, element, options]);

  return isIntersecting;
}
