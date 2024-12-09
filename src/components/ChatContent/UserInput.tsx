import React, { useCallback, useState } from "react";
import { Text, Container, Button, Flex, IconButton } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import { RetryForm } from "../ChatForm";
import styles from "./ChatContent.module.css";
import { Pencil2Icon } from "@radix-ui/react-icons";
import {
  ProcessedUserMessageContentWithImages,
  UserMessageContentWithImage,
  type UserMessage,
} from "../../services/refact";
import { takeWhile } from "../../utils";
import { DialogImage } from "../DialogImage";

export type UserInputProps = {
  children: UserMessage["content"];
  messageIndex: number;
  // maybe add images argument ?
  onRetry: (index: number, question: UserMessage["content"]) => void;
  // disableRetry?: boolean;
};

export const UserInput: React.FC<UserInputProps> = ({
  messageIndex,
  children,
  onRetry,
}) => {
  const [showTextArea, setShowTextArea] = useState(false);
  const [isEditButtonVisible, setIsEditButtonVisible] = useState(false);
  // const ref = React.useRef<HTMLButtonElement>(null);

  const handleSubmit = useCallback(
    (value: UserMessage["content"]) => {
      onRetry(messageIndex, value);
      setShowTextArea(false);
    },
    [messageIndex, onRetry],
  );

  const handleShowTextArea = useCallback(
    (value: boolean) => {
      setShowTextArea(value);
      if (isEditButtonVisible) {
        setIsEditButtonVisible(false);
      }
    },
    [isEditButtonVisible],
  );

  // const lines = children.split("\n"); // won't work if it's an array
  const elements = process(children);
  const isString = typeof children === "string";
  const linesLength = isString ? children.split("\n").length : Infinity;

  return (
    <Container position="relative" pt="1">
      {showTextArea ? (
        <RetryForm
          onSubmit={handleSubmit}
          // TODO
          // value={children}
          value={children}
          onClose={() => handleShowTextArea(false)}
        />
      ) : (
        <Flex
          direction="row"
          // checking for the length of the lines to determine the position of the edit button
          gap={linesLength <= 2 ? "2" : "1"}
          // TODO: what is it's a really long sentence or word with out new lines?
          align={linesLength <= 2 ? "center" : "end"}
          my="1"
          onMouseEnter={() => setIsEditButtonVisible(true)}
          onMouseLeave={() => setIsEditButtonVisible(false)}
        >
          <Button
            // ref={ref}
            variant="soft"
            size="4"
            className={styles.userInput}
            // TODO: should this work?
            // onClick={() => handleShowTextArea(true)}
            asChild
          >
            <div>{elements}</div>
          </Button>
          <IconButton
            title="Edit message"
            variant="soft"
            size={"2"}
            onClick={() => handleShowTextArea(true)}
            style={{
              opacity: isEditButtonVisible ? 1 : 0,
              visibility: isEditButtonVisible ? "visible" : "hidden",
              transition: "opacity 0.15s, visibility 0.15s",
            }}
          >
            <Pencil2Icon width={15} height={15} />
          </IconButton>
        </Flex>
      )}
    </Container>
  );
};

function process(items: UserInputProps["children"]) {
  if (typeof items !== "string") {
    return processUserInputArray(items);
  }

  return processLines(items.split("\n"));
}

function processLines(
  lines: string[],
  processedLinesMemo: JSX.Element[] = [],
): JSX.Element[] {
  if (lines.length === 0) return processedLinesMemo;

  const [head, ...tail] = lines;
  const nextBackTicksIndex = tail.findIndex((l) => l.startsWith("```"));
  const key = `line-${processedLinesMemo.length + 1}`;

  if (!head.startsWith("```") || nextBackTicksIndex === -1) {
    const processedLines = processedLinesMemo.concat(
      <Text
        size="2"
        as="div"
        key={key}
        wrap="balance"
        className={styles.break_word}
      >
        {head}
      </Text>,
    );
    return processLines(tail, processedLines);
  }

  const endIndex = nextBackTicksIndex + 1;

  const code = [head].concat(tail.slice(0, endIndex)).join("\n");
  const processedLines = processedLinesMemo.concat(
    <Markdown key={key}>{code}</Markdown>,
  );

  const next = tail.slice(endIndex);
  return processLines(next, processedLines);
}

function isUserContentImage(
  item: UserMessageContentWithImage | ProcessedUserMessageContentWithImages,
) {
  return (
    ("m_type" in item && item.m_type.startsWith("image/")) ||
    ("type" in item && item.type === "image_url")
  );
}

function processUserInputArray(
  items: (
    | UserMessageContentWithImage
    | ProcessedUserMessageContentWithImages
  )[],
  memo: JSX.Element[] = [],
) {
  if (items.length === 0) return memo;
  const [head, ...tail] = items;

  if ("type" in head && head.type === "text") {
    const processedLines = processLines(head.text.split("\n"));
    return processUserInputArray(tail, memo.concat(processedLines));
  }

  if ("m_type" in head && head.m_type === "text") {
    const processedLines = processLines(head.m_content.split("\n"));
    return processUserInputArray(tail, memo.concat(processedLines));
  }

  const isImage = isUserContentImage(head);

  if (!isImage) return processUserInputArray(tail, memo);

  const imagesInTail = takeWhile(tail, isUserContentImage);
  const nextTail = tail.slice(imagesInTail.length);
  const images = [head, ...imagesInTail];
  const elem = (
    <Flex key={`user-image-images-${memo.length}`} gap="2" wrap="wrap" my="2">
      {images.map((image, index) => {
        if ("type" in image && image.type === "image_url") {
          const key = `user-input${memo.length}-${image.type}-${index}`;
          const content = image.image_url.url;
          return <DialogImage src={content} key={key} />;
        }
        if ("m_type" in image && image.m_type.startsWith("image/")) {
          const key = `user-input${memo.length}-${image.m_type}-${index}`;
          const content = `data:${image.m_type};base64,${image.m_content}`;
          return <DialogImage src={content} key={key} />;
        }
        return null;
      })}
    </Flex>
  );

  return processUserInputArray(nextTail, memo.concat(elem));
}
