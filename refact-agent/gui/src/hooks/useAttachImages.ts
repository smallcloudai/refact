import { useCallback } from "react";
import {
  isUserMessage,
  type UserMessage,
  type UserMessageContentWithImage,
} from "../services/refact/types";
import { useAppSelector } from "../hooks/useAppSelector";
import { selectAllImages } from "../features/AttachedImages/imagesSlice";
import { lastIndex } from "../utils/takeFromLast";

export function useAttachImages() {
  const attachedImages = useAppSelector(selectAllImages);
  const maybeAddImagesToContent = useCallback(
    (question: string): UserMessage["ftm_content"] => {
      if (attachedImages.length === 0) {
        return question;
      }

      const images = attachedImages.reduce<UserMessageContentWithImage[]>(
        (acc, image) => {
          if (typeof image.content !== "string") return acc;
          return [
            ...acc,
            {
              type: "image_url",
              image_url: { url: image.content },
            },
          ];
        },
        [],
      );

      if (images.length === 0) {
        return question;
      }

      return [...images, { type: "text", text: question }];
    },
    [attachedImages],
  );

  const maybeAddImagesToMessages = useCallback(
    (messages: { ftm_role: string; ftm_content: unknown }[]) => {
      const lastUserMessageIndex = lastIndex(messages, isUserMessage);
      if (lastUserMessageIndex === -1) return messages;
      const messagesWithImages = messages.map((message, index) => {
        if (index !== lastUserMessageIndex) return message;
        if (!isUserMessage(message)) return message;
        if (typeof message.ftm_content !== "string") return message;
        const content = maybeAddImagesToContent(message.ftm_content);
        return {
          ...message,
          ftm_content: content,
        };
      });
      return messagesWithImages;
    },
    [maybeAddImagesToContent],
  );

  return { maybeAddImagesToContent, maybeAddImagesToMessages };
}
