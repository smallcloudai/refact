import { KeyboardEvent, KeyboardEventHandler } from "react";

export const useOnPressedEnter =
  (onPress: KeyboardEventHandler) => (event: KeyboardEvent) => {
    if (event.key === "Enter" && !event.shiftKey) {
      onPress(event);
    }
  };
