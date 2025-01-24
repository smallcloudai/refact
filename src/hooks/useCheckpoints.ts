import { useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import { selectIsCheckpointsPopupIsVisible } from "../features/Checkpoints/checkpointsSlice";

export const useCheckpoints = () => {
  const isCheckpointsPopupVisible = useAppSelector(
    selectIsCheckpointsPopupIsVisible,
  );

  const shouldCheckpointsPopupBeShown = useMemo(() => {
    // TODO: show if isVisible and files data is not empty
    return isCheckpointsPopupVisible;
  }, [isCheckpointsPopupVisible]);
  return { shouldCheckpointsPopupBeShown };
};
