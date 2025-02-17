import {
  useCallback,
  type KeyboardEvent,
  type KeyboardEventHandler,
} from "react";
import { useAppSelector } from "./useAppSelector";
import { selectSubmitOption } from "../features/Config/configSlice";

export const useOnPressedEnter = (onPress: KeyboardEventHandler) => {
  const submitOption = useAppSelector(selectSubmitOption);

  const onKeyPress = useCallback(
    (event: KeyboardEvent) => {
      if (!submitOption && event.key === "Enter" && !event.shiftKey) {
        onPress(event);
      } else if (submitOption && event.key === "Enter" && event.shiftKey) {
        onPress(event);
      }
    },
    [submitOption, onPress],
  );

  return onKeyPress;
};
