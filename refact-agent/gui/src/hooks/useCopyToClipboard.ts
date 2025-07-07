import { useCallback } from "react";

import { fallbackCopying } from "../utils/fallbackCopying";

export const useCopyToClipboard = () => {
  const handleCopy = useCallback((text: string) => {
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
    if (window.navigator?.clipboard?.writeText) {
      void window.navigator.clipboard.writeText(text).catch(() => {
        // eslint-disable-next-line no-console
        console.log("failed to copy to clipboard");
      });
    } else {
      fallbackCopying(text);
    }
  }, []);

  return handleCopy;
};
