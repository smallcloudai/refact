import { useCallback, useEffect } from "react";
import { useAppSelector } from "./useAppSelector";
import { useAppDispatch } from "./useAppDispatch";
import {
  selectAllImages,
  removeImageByIndex,
  addImage,
  type ImageFile,
  resetAttachedImagesSlice,
} from "../features/AttachedImages";
import { setError } from "../features/Errors/errorsSlice";
import { setInformation } from "../features/Errors/informationSlice";
import { useCapsForToolUse } from "./useCapsForToolUse";

export function useAttachedImages() {
  const images = useAppSelector(selectAllImages);
  const { isMultimodalitySupportedForCurrentModel } = useCapsForToolUse();
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

  const processAndInsertImages = useCallback(
    (files: File[]) => {
      if (files.length > 5) {
        handleError("You can only upload 5 images at a time");
        return;
      } else {
        void processImages(files, insertImage, handleError, handleWarning);
      }
    },
    [handleError, handleWarning, insertImage],
  );

  useEffect(() => {
    if (!isMultimodalitySupportedForCurrentModel) {
      const action = resetAttachedImagesSlice();
      dispatch(action);
    }
  }, [isMultimodalitySupportedForCurrentModel, dispatch]);

  return {
    images,
    setError: handleError,
    setWarning: handleWarning,
    insertImage,
    removeImage,
    processAndInsertImages,
  };
}

async function processImages(
  files: File[],
  onSuccess: (image: ImageFile) => void,
  onError: (reason: string) => void,
  onAbort: (reason: string) => void,
) {
  for (const file of files) {
    if (file.type !== "image/jpeg" && file.type !== "image/png") {
      onError(`file ${file.type} is not a supported. Use jpeg or png`);
    } else {
      try {
        const scaledImage = await scaleImage(file, 800);
        const fileForChat = {
          name: file.name,
          content: scaledImage,
          type: file.type,
        };
        onSuccess(fileForChat);
      } catch (error) {
        if (error === "abort") {
          onAbort(`file ${file.name} reading was aborted`);
        } else {
          onError(`file ${file.name} processing has failed`);
        }
      }
    }
  }
}
function scaleImage(file: File, maxSize: number): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const img = new Image();
      img.onload = () => {
        const canvas = document.createElement("canvas");
        const ctx = canvas.getContext("2d");
        if (ctx === null) {
          reject(`canvas.getContext("2d"), returned null`);
        }

        let width = img.width;
        let height = img.height;

        if (width > height && width > maxSize) {
          height = Math.round((height *= maxSize / width));
          width = maxSize;
        } else if (height >= width && height > maxSize) {
          width = Math.round((width *= maxSize / height));
          height = maxSize;
        }

        canvas.width = width;
        canvas.height = height;
        ctx?.drawImage(img, 0, 0, width, height);

        resolve(canvas.toDataURL(file.type));
      };
      img.onerror = reject;
      img.src = reader.result as string;
    };

    reader.onabort = () => reject("aborted");
    reader.onerror = () => reject("error");
    reader.readAsDataURL(file);
  });
}
