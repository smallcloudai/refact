import { useEffect, useState } from "react";

export function useAutoFocusOnce(options?: FocusOptions) {
  const [focus, setFocus] = useState<boolean>(true);
  useEffect(() => {
    if (focus) {
      setFocus(false);
    }
  }, [focus, options]);

  return focus;
}
