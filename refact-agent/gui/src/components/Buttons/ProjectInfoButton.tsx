import React, { useCallback } from "react";
import { useAppDispatch, useAppSelector } from "../../hooks";
import {
  selectChatId,
  selectIncludeProjectInfo,
  selectMessages,
  setIncludeProjectInfo,
} from "../../features/Chat/Thread";
import { Button, Flex, HoverCard, Text } from "@radix-ui/themes";

export const ProjectInfoButton: React.FC = () => {
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const messages = useAppSelector(selectMessages);
  const includeProjectInfo = useAppSelector(selectIncludeProjectInfo);

  const handleChange = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      dispatch(
        setIncludeProjectInfo({
          chatId,
          value: !includeProjectInfo,
        }),
      );
    },
    [dispatch, chatId, includeProjectInfo],
  );

  // Only show when starting a new chat (no messages yet)
  if (messages.length > 0) {
    return null;
  }

  return (
    <Flex gap="2" align="center">
      <HoverCard.Root>
        <HoverCard.Trigger>
          <Button
            size="1"
            onClick={handleChange}
            variant={includeProjectInfo ? "solid" : "outline"}
          >
            📁 Project
          </Button>
        </HoverCard.Trigger>
        <HoverCard.Content size="2" maxWidth="300px" side="top">
          <Text as="p" size="2">
            When enabled, extra project context information will be included at
            the start of the chat to help the AI understand your codebase
            better.
          </Text>
          <Text as="p" color="yellow" size="1" mt="2">
            ⚠️ Note: This can consume a significant amount of tokens initially.
          </Text>
          <Text as="p" color="gray" size="1" mt="1">
            This option is only available when starting a new chat.
          </Text>
        </HoverCard.Content>
      </HoverCard.Root>
    </Flex>
  );
};
