import { ResetIcon } from "@radix-ui/react-icons";
import { IconButton } from "@radix-ui/themes";
import { useAppDispatch } from "../../hooks";
import { setIsCheckpointsPopupIsVisible } from "./checkpointsSlice";
import { useCallback } from "react";

export const CheckpointButton = () => {
  const dispatch = useAppDispatch();

  const handleClick = useCallback(() => {
    dispatch(setIsCheckpointsPopupIsVisible(true));
  }, [dispatch]);

  return (
    <IconButton
      size="2"
      variant="soft"
      title="Revert agent changes"
      onClick={handleClick}
    >
      <ResetIcon />
    </IconButton>
  );
};
