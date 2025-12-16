import React, { useCallback } from "react";
import { Flex, Text, IconButton, Card, Badge } from "@radix-ui/themes";
import {
  Cross1Icon,
  ClockIcon,
  LightningBoltIcon,
} from "@radix-ui/react-icons";
import { useAppDispatch } from "../../hooks";
import { dequeueUserMessage, QueuedUserMessage } from "../../features/Chat";
import styles from "./ChatContent.module.css";
import classNames from "classnames";

type QueuedMessageProps = {
  queuedMessage: QueuedUserMessage;
  position: number;
};

function getMessagePreview(message: QueuedUserMessage["message"]): string {
  if (typeof message.content === "string") {
    return message.content;
  }
  // Handle multimodal content
  const textPart = message.content.find(
    (part) => "type" in part && part.type === "text",
  );
  if (textPart && "text" in textPart) {
    return textPart.text;
  }
  return "[Image attachment]";
}

export const QueuedMessage: React.FC<QueuedMessageProps> = ({
  queuedMessage,
  position,
}) => {
  const dispatch = useAppDispatch();
  const isPriority = queuedMessage.priority;

  const handleCancel = useCallback(() => {
    dispatch(dequeueUserMessage({ queuedId: queuedMessage.id }));
  }, [dispatch, queuedMessage.id]);

  const preview = getMessagePreview(queuedMessage.message);

  return (
    <Card
      className={classNames(styles.queuedMessage, {
        [styles.queuedMessagePriority]: isPriority,
      })}
    >
      <Flex gap="2" align="start" justify="between">
        <Flex gap="2" align="center" style={{ flex: 1, minWidth: 0 }}>
          <Badge color={isPriority ? "blue" : "amber"} variant="soft" size="1">
            {isPriority ? (
              <LightningBoltIcon width={12} height={12} />
            ) : (
              <ClockIcon width={12} height={12} />
            )}
            {position}
          </Badge>
          <Text
            size="2"
            color="gray"
            className={styles.queuedMessageText}
            title={preview}
          >
            {preview}
          </Text>
        </Flex>
        <IconButton
          size="1"
          variant="ghost"
          color="gray"
          onClick={handleCancel}
          title="Cancel queued message"
        >
          <Cross1Icon width={14} height={14} />
        </IconButton>
      </Flex>
    </Card>
  );
};

export default QueuedMessage;
