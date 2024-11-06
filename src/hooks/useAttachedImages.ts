import { useCallback } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import {
  selectAllImages,
  removeImageByIndex,
  addImage,
  type ImageFile,
} from "../features/AttachedImages";
import { setError } from "../features/Errors/errorsSlice";
import { setInformation } from "../features/Errors/informationSlice";

export function useAttachedImages() {
  const images = useAppSelector(selectAllImages);
  const dispatch = useAppDispatch();

  const removeImage = useCallback(
    (index: number) => {
      const action = removeImageByIndex(index);
      dispatch(action);
    },
    [dispatch],
  );

  const insertImage = useCallback(
    (file: ImageFile) => {
      const action = addImage(file);
      dispatch(action);
    },
    [dispatch],
  );

  const handleError = useCallback(
    (error: string) => {
      const action = setError(error);
      dispatch(action);
    },
    [dispatch],
  );

  const handleWarning = useCallback(
    (warning: string) => {
      const action = setInformation(warning);
      dispatch(action);
    },
    [dispatch],
  );

  return {
    images,
    setError: handleError,
    setWarning: handleWarning,
    insertImage,
    removeImage,
  };
}
