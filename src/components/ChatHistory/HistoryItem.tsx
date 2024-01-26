import React from "react";
import { Card, Flex, Text, Box } from "@radix-ui/themes";
import type { ChatHistoryItem } from "../../hooks/useChatHistory";
import { ChatBubbleIcon } from "@radix-ui/react-icons";
import { CloseButton } from "../Buttons/Buttons";

export const HistoryItem: React.FC<{
  chat: ChatHistoryItem;
  onClick: (id: string) => void;
  onDelete: (id: string) => void;
}> = ({ chat, onClick, onDelete }) => {
  const dateCreated = new Date(chat.createdAt);
  const dateTimeString = dateCreated.toLocaleString();
  return (
    <Box style={{ position: "relative" }}>
      <Card
        style={{ width: "240px", marginBottom: "2px" }}
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
      <CloseButton
        size="1"
        // needs to be smaller
        style={{
          position: "absolute",
          right: "6px",
          top: "6px",
        }}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onDelete(chat.id);
        }}
        iconSize={10}
        title="delete chat"
      />
    </Box>
  );
};
