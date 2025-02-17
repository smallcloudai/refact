import React, { useCallback, useState } from "react";
import { Avatar, Button, Flex, Box } from "@radix-ui/themes";
import { FileRejection, useDropzone } from "react-dropzone";
import { TextArea } from "../TextArea";
import { useOnPressedEnter } from "../../hooks/useOnPressedEnter";
import { Form } from "./Form";

import { useAgentUsage, useAppSelector, useCapsForToolUse } from "../../hooks";
import { selectSubmitOption } from "../../features/Config/configSlice";
import {
  ProcessedUserMessageContentWithImages,
  UserImage,
  UserMessage,
} from "../../services/refact";
import { ImageIcon, CrossCircledIcon } from "@radix-ui/react-icons";
import { useAttachedImages } from "../../hooks/useAttachedImages";

function getTextFromUserMessage(messages: UserMessage["content"]): string {
  if (typeof messages === "string") return messages;
  return messages.reduce<string>((acc, message) => {
    if ("m_type" in message && message.m_type === "text")
      return acc + message.m_content;
    if ("type" in message && message.type === "text") return acc + message.text;
    return acc;
  }, "");
}

function getImageFromUserMessage(
  messages: UserMessage["content"],
): (UserImage | ProcessedUserMessageContentWithImages)[] {
  if (typeof messages === "string") return [];

  const images = messages.reduce<
    (UserImage | ProcessedUserMessageContentWithImages)[]
  >((acc, message) => {
    if ("m_type" in message && message.m_type.startsWith("image/"))
      return [...acc, message];
    if ("type" in message && message.type === "image_url")
      return [...acc, message];
    return acc;
  }, []);

  return images;
}

function getImageContent(
  image: UserImage | ProcessedUserMessageContentWithImages,
) {
  if ("type" in image) return image.image_url.url;
  const base64 = `data:${image.m_type};base64,${image.m_content}`;
  return base64;
}

export const RetryForm: React.FC<{
  // value: string;
  value: UserMessage["content"];
  onSubmit: (value: UserMessage["content"]) => void;
  onClose: () => void;
}> = (props) => {
  const shiftEnterToSubmit = useAppSelector(selectSubmitOption);
  const { disableInput } = useAgentUsage();
  const { isMultimodalitySupportedForCurrentModel } = useCapsForToolUse();
  const inputText = getTextFromUserMessage(props.value);
  const inputImages = getImageFromUserMessage(props.value);
  const [textValue, onChangeTextValue] = useState(inputText);
  const [imageValue, onChangeImageValue] = useState(inputImages);

  const addImage = useCallback((image: UserImage) => {
    onChangeImageValue((prev) => {
      return [...prev, image];
    });
  }, []);

  const closeAndReset = () => {
    onChangeImageValue(inputImages);
    onChangeTextValue(inputText);
    props.onClose();
  };

  const handleRetry = () => {
    if (disableInput) return;
    const trimmedText = textValue.trim();
    if (imageValue.length === 0 && trimmedText.length > 0) {
      props.onSubmit(trimmedText);
    } else if (trimmedText.length > 0) {
      const text = {
        type: "text" as const,
        text: textValue.trim(),
      };
      props.onSubmit([text, ...imageValue]);
    }
  };

  const onPressedEnter = useOnPressedEnter(handleRetry);

  const handleOnKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (shiftEnterToSubmit && !event.shiftKey && event.key === "Enter") {
        onChangeTextValue(textValue + "\n");
        return;
      }
      onPressedEnter(event);
    },
    [onPressedEnter, shiftEnterToSubmit, textValue],
  );

  const handleRemove = useCallback((index: number) => {
    onChangeImageValue((prev) => {
      return prev.filter((_, i) => i !== index);
    });
  }, []);

  return (
    <Form
      onSubmit={(event) => {
        event.preventDefault();
        handleRetry();
      }}
    >
      <TextArea
        value={textValue}
        onChange={(event) => onChangeTextValue(event.target.value)}
        onKeyDown={handleOnKeyDown}
      />

      {imageValue.length > 0 && (
        <Flex
          px="2"
          py="4"
          wrap="wrap"
          direction="row"
          align="center"
          justify="center"
          style={{
            backgroundColor: "var(--color-surface)",
          }}
        >
          {imageValue.map((image, index) => {
            return (
              <MyImage
                key={`retry-user-image-${index}`}
                image={getImageContent(image)}
                onRemove={() => handleRemove(index)}
              />
            );
          })}
        </Flex>
      )}

      <Flex
        align="center"
        justify="center"
        gap="1"
        direction="row"
        p="2"
        wrap="wrap"
        style={{
          backgroundColor: "var(--color-surface)",
        }}
      >
        <Button
          color="grass"
          variant="surface"
          size="1"
          type="submit"
          disabled={disableInput}
          title={
            disableInput
              ? "You have reached your usage limit of 20 messages a day. You can use agent again tomorrow, or upgrade to PRO."
              : ""
          }
        >
          Submit
        </Button>
        <Button
          variant="surface"
          color="tomato"
          size="1"
          onClick={closeAndReset}
        >
          Cancel
        </Button>

        {isMultimodalitySupportedForCurrentModel && (
          <MyDropzone addImage={addImage} />
        )}
      </Flex>
    </Form>
  );
};

const MyDropzone: React.FC<{
  addImage: (image: UserImage) => void;
}> = ({ addImage }) => {
  const { setError, setWarning } = useAttachedImages();

  const onDrop = useCallback(
    (acceptedFiles: File[], fileRejections: FileRejection[]) => {
      acceptedFiles.forEach((file) => {
        const reader = new FileReader();
        reader.onabort = () =>
          setWarning(`file ${file.name} reading was aborted`);
        reader.onerror = () => setError(`file ${file.name} reading has failed`);
        reader.onload = () => {
          if (typeof reader.result === "string") {
            const image: UserImage = {
              type: "image_url",
              image_url: { url: reader.result },
            };
            addImage(image);
          }
        };
        reader.readAsDataURL(file);
      });

      if (fileRejections.length) {
        const rejectedFileMessage = fileRejections.map((file) => {
          const err = file.errors.reduce<string>((acc, cur) => {
            return acc + `${cur.code} ${cur.message}\n`;
          }, "");
          return `Could not attach ${file.file.name}: ${err}`;
        });
        setError(rejectedFileMessage.join("\n"));
      }
    },
    [addImage, setError, setWarning],
  );

  const { getRootProps, getInputProps, open } = useDropzone({
    onDrop,
    disabled: false,
    noClick: true,
    noKeyboard: true,
    accept: {
      "image/*": [],
    },
  });

  return (
    <div {...getRootProps()}>
      <input {...getInputProps()} style={{ display: "none" }} />
      <Button
        size="1"
        variant="surface"
        color="gray"
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          open();
        }}
      >
        Add images
      </Button>
    </div>
  );
};

const MyImage: React.FC<{ image: string; onRemove: () => void }> = ({
  image,
  onRemove,
}) => {
  return (
    <Box position="relative">
      <Button
        variant="ghost"
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onRemove();
        }}
      >
        <CrossCircledIcon
          width="16"
          color="gray"
          style={{
            position: "absolute",
            right: "calc(var(--space-2) * -1)",
            top: "calc(var(--space-2) * -1)",
          }}
        />
        <Avatar src={image} size="4" fallback={<ImageIcon />} />
      </Button>
    </Box>
  );
};
