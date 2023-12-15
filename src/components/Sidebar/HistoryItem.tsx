import React from "react";
import { Card, Flex, Text } from "@radix-ui/themes";
import { ChatHistoryItem } from "../../hooks/useChatHistory";
import { ChatBubbleIcon } from "@radix-ui/react-icons";

export const HistoryItem: React.FC<{
  chat: ChatHistoryItem;
  onClick: (id: string) => void;
}> = ({ chat, onClick }) => {
  const dateCreated = new Date(chat.createdAt);
  const dateTimeString = dateCreated.toLocaleString();
  return (
    <Card
      style={{ width: "240px", marginBottom: "2px" }}
      variant="surface"
      asChild
    >
      <button
        onClick={() => {
          console.log("Clicked");
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
  );
};
