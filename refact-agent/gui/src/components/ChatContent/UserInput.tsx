import {
  Box,
  Button,
  Container,
  Flex,
  IconButton,
  Text,
} from "@radix-ui/themes";
import React, { useMemo, useState } from "react";
import {
  ProcessedUserMessageContentWithImages,
  UserMessageContentWithImage,
  type UserMessage,
} from "../../services/refact";
import { takeWhile } from "../../utils";
// import { RetryForm } from "../ChatForm";
import { DialogImage } from "../DialogImage";
import { Markdown } from "../Markdown";
import styles from "./ChatContent.module.css";
import { Reveal } from "../Reveal";
import { ArrowLeftIcon, ArrowRightIcon } from "@radix-ui/react-icons";
import { BranchIcon } from "../../images";
import classNames from "classnames";

export type UserInputProps = {
  children: UserMessage["ftm_content"];
  // messageIndex: number;
  // maybe add images argument ?
  // onRetry: (index: number, question: UserMessage["content"]) => void;
  // disableRetry?: boolean;
  branch?: NodeSelectButtonsProps;
};

export const UserInput: React.FC<UserInputProps> = ({
  // messageIndex,
  children,
  // onRetry,
  branch,
}) => {
  // const [showTextArea, setShowTextArea] = useState(false);
  const [isEditButtonVisible, setIsEditButtonVisible] = useState(false);

  // const handleSubmit = useCallback(
  //   (value: UserMessage["content"]) => {
  //     onRetry(messageIndex, value);
  //     setShowTextArea(false);
  //   },
  //   [messageIndex, onRetry],
  // );

  // const handleShowTextArea = useCallback(
  //   (value: boolean) => {
  //     setShowTextArea(value);
  //     if (isEditButtonVisible) {
  //       setIsEditButtonVisible(false);
  //     }
  //   },
  //   [isEditButtonVisible],
  // );

  // const lines = children.split("\n"); // won't work if it's an array
  const elements = process(children);
  const isString = typeof children === "string";
  const linesLength = isString ? children.split("\n").length : Infinity;

  // const checkpointsFromMessage = useMemo(() => {
  //   const maybeUserMessage = messages[messageIndex];
  //   if (!isUserMessage(maybeUserMessage)) return null;
  //   return maybeUserMessage.checkpoints;
  // }, [messageIndex, messages]);

  const isCompressed = useMemo(() => {
    if (typeof children !== "string") return false;
    return children.startsWith("🗜️ ");
  }, [children]);

  return (
    <Container position="relative" pt="1">
      {isCompressed ? (
        <Reveal defaultOpen={false}>
          <Flex direction="row" my="1" className={styles.userInput}>
            {elements}
          </Flex>
        </Reveal>
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
          {/** todo, no button needed */}
          <Button
            // ref={ref}
            variant="soft"
            size="4"
            className={classNames(styles.userInput, !children && styles.empty)}
            // TODO: should this work?
            // onClick={() => handleShowTextArea(true)}
            asChild
          >
            <div>{elements}</div>
          </Button>
          <Flex
            direction={linesLength <= 3 ? "row" : "column"}
            gap="1"
            style={{
              opacity: isEditButtonVisible ? 1 : 0,
              visibility: isEditButtonVisible ? "visible" : "hidden",
              transition: "opacity 0.15s, visibility 0.15s",
            }}
          >
            {/* {checkpointsFromMessage && checkpointsFromMessage.length > 0 && (
              <CheckpointButton
                checkpoints={checkpointsFromMessage}
                messageIndex={messageIndex}
              />
            )} */}
            {/* <IconButton
              title="Edit message"
              variant="soft"
              size={"2"}
              onClick={() => handleShowTextArea(true)}
            >
              <Pencil2Icon width={15} height={15} />
            </IconButton> */}
          </Flex>
        </Flex>
      )}
      {branch && <NodeSelectButtons {...branch} />}
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
        key={`text-${key}`}
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
    <Markdown key={`markdown-${key}`}>{code}</Markdown>,
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
    const elem = (
      <Box key={`multimodal-text-${memo.length}`}>{processedLines}</Box>
    );
    return processUserInputArray(tail, memo.concat(elem));
  }

  if ("m_type" in head && head.m_type === "text") {
    const processedLines = processLines(head.m_content.split("\n"));
    const elem = (
      <Box key={`multimodal-text-${memo.length}`}>{processedLines}</Box>
    );
    return processUserInputArray(tail, memo.concat(elem));
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

export type NodeSelectButtonsProps = {
  onForward: () => void;
  onBackward: () => void;
  currentNode: number;
  totalNodes: number;
};

const NodeSelectButtons: React.FC<NodeSelectButtonsProps> = ({
  onForward,
  onBackward,
  currentNode,
  totalNodes,
}) => {
  if (totalNodes === 1 && currentNode === 0) {
    return (
      <Box mt="2">
        <IconButton
          variant="ghost"
          size="1"
          radius="large"
          onClick={onForward}
          title="create a new branch"
        >
          <BranchIcon />
        </IconButton>
      </Box>
    );
  }

  return (
    <Flex gap="2" justify="start" mt="2">
      <IconButton
        variant="ghost"
        size="1"
        disabled={currentNode === 0}
        radius="large"
        onClick={onBackward}
      >
        <ArrowLeftIcon />
      </IconButton>
      <Text size="1">
        {currentNode + 1} / {totalNodes}
      </Text>
      <IconButton
        variant="ghost"
        size="1"
        disabled={currentNode === totalNodes}
        onClick={onForward}
        radius="large"
      >
        <ArrowRightIcon />
      </IconButton>
    </Flex>
  );
};
