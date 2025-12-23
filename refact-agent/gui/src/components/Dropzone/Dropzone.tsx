import React, { createContext, useCallback } from "react";
import { Button, Slot, IconButton, Flex } from "@radix-ui/themes";
import { Cross1Icon, ImageIcon } from "@radix-ui/react-icons";
import { DropzoneInputProps, FileRejection, useDropzone } from "react-dropzone";
import { useAttachedImages } from "../../hooks/useAttachedImages";
import { TruncateLeft } from "../Text";
import { telemetryApi } from "../../services/refact/telemetry";
import { useCapsForToolUse } from "../../hooks";
import { useAttachedFiles } from "../ChatForm/useCheckBoxes";

export const FileUploadContext = createContext<{
  open: () => void;

  getInputProps: (props?: DropzoneInputProps) => DropzoneInputProps;
}>({
  open: () => ({}),
  getInputProps: () => ({}),
});

export const DropzoneProvider: React.FC<
  React.PropsWithChildren<{ asChild?: boolean }>
> = ({ asChild, ...props }) => {
  const { setError, processAndInsertImages } = useAttachedImages();
  const { isMultimodalitySupportedForCurrentModel } = useCapsForToolUse();

  const onDrop = useCallback(
    (acceptedFiles: File[], fileRejections: FileRejection[]): void => {
      if (!isMultimodalitySupportedForCurrentModel) return;
      processAndInsertImages(acceptedFiles);

      if (fileRejections.length) {
        const rejectedFileMessage = fileRejections.map((file) => {
          const err = file.errors.reduce<string>((acc, cur) => {
            return acc + `${cur.code} ${cur.message}\n`;
          }, "");
          return `could not attach ${file.file.name}: ${err}`;
        });
        setError(rejectedFileMessage.join("\n"));
      }
    },
    [processAndInsertImages, setError, isMultimodalitySupportedForCurrentModel],
  );

  // TODO: disable when chat is busy
  const dropzone = useDropzone({
    disabled: false,
    noClick: true,
    noKeyboard: true,
    accept: {
      // "image/*": []
      // "image/apng": [],
      // "image/avif": [],
      // "image/gif": [],
      "image/jpeg": [],
      "image/png": [],
      // "image/svg+xml": [],
      // "image/webp": [],
      // "image/bmp": [],
      // "image/x-icon": [],
      // "image/tiff": []
    },
    onDrop,
  });

  const Comp = asChild ? Slot : "div";

  return (
    <FileUploadContext.Provider
      value={{
        open: dropzone.open,
        getInputProps: dropzone.getInputProps,
      }}
    >
      <Comp {...dropzone.getRootProps()} {...props} />
    </FileUploadContext.Provider>
  );
};

export const DropzoneConsumer = FileUploadContext.Consumer;

export const AttachImagesButton = () => {
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();
  const attachFileOnClick = useCallback(
    (
      event: { preventDefault: () => void; stopPropagation: () => void },
      open: () => void,
    ) => {
      event.preventDefault();
      event.stopPropagation();
      open();
      void sendTelemetryEvent({
        scope: `addImage/button`, // add drag&drop and clipboard
        success: true,
        error_message: "",
      });
    },
    [sendTelemetryEvent],
  );
  return (
    <DropzoneConsumer>
      {({ open, getInputProps }) => {
        const inputProps = getInputProps();
        return (
          <>
            <input {...inputProps} style={{ display: "none" }} />
            <IconButton
              variant="ghost"
              size="1"
              title="Attach images"
              disabled={inputProps.disabled}
              onClick={(event) => {
                attachFileOnClick(event, open);
              }}
            >
              <ImageIcon />
            </IconButton>
          </>
        );
      }}
    </DropzoneConsumer>
  );
};

type FileListProps = {
  attachedFiles: ReturnType<typeof useAttachedFiles>;
};
export const FileList: React.FC<FileListProps> = ({ attachedFiles }) => {
  const { images, removeImage } = useAttachedImages();
  if (images.length === 0 && attachedFiles.files.length === 0) return null;
  return (
    <Flex wrap="wrap" gap="1" py="2" data-testid="attached_file_list">
      {images.map((file, index) => {
        const key = `image-${file.name}-${index}`;
        return (
          <FileButton
            key={key}
            onClick={() => removeImage(index)}
            fileName={file.name}
          />
        );
      })}
      {attachedFiles.files.map((file, index) => {
        const key = `file-${file.path}-${index}`;
        return (
          <FileButton
            key={key}
            fileName={file.name}
            onClick={() => attachedFiles.removeFile(file)}
          />
        );
      })}
    </Flex>
  );
};

const FileButton: React.FC<{ fileName: string; onClick: () => void }> = ({
  fileName,
  onClick,
}) => {
  return (
    <Button
      variant="soft"
      radius="full"
      size="1"
      onClick={onClick}
      style={{ maxWidth: "100%" }}
    >
      <TruncateLeft wrap="wrap">{fileName}</TruncateLeft>{" "}
      <Cross1Icon width="10" style={{ flexShrink: 0 }} />
    </Button>
  );
};
