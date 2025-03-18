import { useState, useCallback, useEffect } from "react";
import { useResizeObserver } from "../../hooks";

function calculateSpace(
  scrollElem: HTMLElement,
  anchorElem: HTMLElement,
  bottomElem: HTMLElement,
) {
  const anchorPosition = anchorElem.offsetTop;
  const topOfBottom = bottomElem.offsetTop;
  const spaceBetween = topOfBottom - anchorPosition;
  const maxSpace = scrollElem.clientHeight;
  return Math.max(maxSpace - spaceBetween, 0);
}

export function useSpaceCalculator(
  scrollElem?: HTMLElement | null,
  innerElem?: HTMLElement | null,
  anchorElem?: HTMLElement | null,
  bottomElem?: HTMLElement | null,
) {
  const [height, setHeight] = useState<number>(bottomElem?.clientHeight ?? 0);
  const calculateAndSetSpace = useCallback(() => {
    if (!scrollElem || !bottomElem || !anchorElem) {
      return;
    }
    const nextHeight = calculateSpace(scrollElem, anchorElem, bottomElem);
    setHeight(nextHeight);
  }, [scrollElem, bottomElem, anchorElem]);

  useResizeObserver(innerElem ?? null, calculateAndSetSpace);
  useEffect(() => {
    calculateAndSetSpace();
  }, [calculateAndSetSpace]);
  return height;
}
