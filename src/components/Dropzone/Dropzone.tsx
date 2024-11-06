import React, { createContext, useCallback } from "react";
import { Button, Slot, IconButton, Flex } from "@radix-ui/themes";
import { Cross1Icon, ImageIcon } from "@radix-ui/react-icons";
import { DropzoneInputProps, FileRejection, useDropzone } from "react-dropzone";
import { useAttachedImages } from "../../hooks/useAttachedImages";
import { TruncateLeft } from "../Text";

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
  const { insertImage, setError, setWarning } = useAttachedImages();

  const onDrop = useCallback(
    (acceptedFiles: File[], fileRejections: FileRejection[]) => {
      acceptedFiles.forEach((file) => {
        const reader = new FileReader();
        reader.onabort = () =>
          setWarning(`file ${file.name} reading was aborted`);

        reader.onerror = () => setError(`file ${file.name} reading has failed`);

        reader.onload = () => {
          const fileForChat = {
            name: file.name,
            content: reader.result,
            type: file.type,
          };
          insertImage(fileForChat);
        };
        reader.readAsDataURL(file);
      });

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
    [insertImage, setError, setWarning],
  );

  // TODO: disable when chat is busy
  const dropzone = useDropzone({
    disabled: false,
    noClick: true,
    noKeyboard: true,
    accept: {
      "image/*": [],
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

export const AttachFileButton = () => {
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
              title="add image"
              disabled={inputProps.disabled}
              onClick={(event) => {
                event.preventDefault();
                event.stopPropagation();
                open();
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

export const FileList = () => {
  const { images, removeImage } = useAttachedImages();
  if (images.length === 0) return null;
  return (
    <Flex wrap="wrap" gap="1" py="2">
      {images.map((file, index) => {
        const key = `image-${file.name}-${index}`;
        return (
          <Button
            // variant="surface"
            // variant="outline"
            variant="soft"
            radius="full"
            key={key}
            size="1"
            onClick={() => removeImage(index)}
            style={{ maxWidth: "100%" }}
          >
            <TruncateLeft wrap="wrap">{file.name}</TruncateLeft>{" "}
            <Cross1Icon width="10" style={{ flexShrink: 0 }} />
          </Button>
        );
      })}
    </Flex>
  );
};
