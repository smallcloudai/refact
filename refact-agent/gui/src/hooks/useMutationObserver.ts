import { useEffect } from "react";

export const useMutationObserver = (
  elem: Node,
  callback: MutationCallback,
  options: MutationObserverInit = {},
) => {
  useEffect(() => {
    const observer = new MutationObserver(callback);
    observer.observe(elem, options);

    return () => observer.disconnect();
  }, [elem, callback, options]);
};
