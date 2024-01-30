import React from "react";
import { Card, Flex, Text, Box } from "@radix-ui/themes";
import type { ChatHistoryItem } from "../../hooks/useChatHistory";
import { ChatBubbleIcon } from "@radix-ui/react-icons";
import { CloseButton } from "../Buttons/Buttons";
import { IconButton } from "@radix-ui/themes";
import { OpenInNewWindowIcon } from "@radix-ui/react-icons";

export const HistoryItem: React.FC<{
  chat: ChatHistoryItem;
  onClick: (id: string) => void;
  onDelete: (id: string) => void;
  onOpenInTab?: (id: string) => void;
}> = ({ chat, onClick, onDelete, onOpenInTab }) => {
  const dateCreated = new Date(chat.createdAt);
  const dateTimeString = dateCreated.toLocaleString();
  return (
    <Box style={{ position: "relative", width: "100%" }}>
      <Card
        style={{ width: "100%", marginBottom: "2px" }}
        variant="surface"
        className="rt-Button"
        asChild
        role="button"
        onClick={() => onClick(chat.id)}
      >
        <button
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onClick(chat.id);
          }}
        >
          <Text
            as="div"
            size="2"
            weight="bold"
            style={{
              textOverflow: "ellipsis",
              overflow: "hidden",
              whiteSpace: "nowrap",
            }}
          >
            {chat.title}
          </Text>

          <Flex justify="between" style={{ marginTop: "8px" }}>
            <Text
              size="1"
              style={{ display: "flex", gap: "4px", alignItems: "center" }}
            >
              <ChatBubbleIcon />{" "}
              {chat.messages.filter((message) => message[0] === "user").length}
            </Text>

            <Text size="1">{dateTimeString}</Text>
          </Flex>
        </button>
      </Card>

      {/**TODO: open in tab button */}
      <Flex
        style={{
          position: "absolute",
          right: "6px",
          top: "6px",
        }}
        gap="1"
        justify="end"
        align="center"
        // justify to flex end
      >
        {/**TODO: open in tab button */}
        {onOpenInTab && (
          <IconButton
            size="1"
            title="open in tab"
            onClick={(event) => {
              event.preventDefault();
              event.stopPropagation();
              onOpenInTab(chat.id);
            }}
            variant="ghost"
          >
            <OpenInNewWindowIcon width="10" height="10" />
          </IconButton>
        )}

        <CloseButton
          size="1"
          // needs to be smaller

          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onDelete(chat.id);
          }}
          iconSize={10}
          title="delete chat"
        />
      </Flex>
    </Box>
  );
};
