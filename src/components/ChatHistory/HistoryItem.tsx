import React from "react";
import { Card, Flex, Text, Box } from "@radix-ui/themes";
// import type { ChatHistoryItem } from "../../hooks/useChatHistory";
import { ChatBubbleIcon } from "@radix-ui/react-icons";
import { CloseButton } from "../Buttons/Buttons";
import { IconButton } from "@radix-ui/themes";
import { OpenInNewWindowIcon } from "@radix-ui/react-icons";
import type { ChatHistoryItem } from "../../features/History/historySlice";
import { isUserMessage } from "../../services/refact";

export const HistoryItem: React.FC<{
  historyItem: ChatHistoryItem;
  onClick: () => void;
  onDelete: (id: string) => void;
  onOpenInTab?: (id: string) => void;
  disabled: boolean;
}> = ({ historyItem, onClick, onDelete, onOpenInTab, disabled }) => {
  const dateCreated = new Date(historyItem.createdAt);
  const dateTimeString = dateCreated.toLocaleString();
  return (
    <Box style={{ position: "relative", width: "100%" }}>
      <Card
        style={{
          width: "100%",
          marginBottom: "2px",
          opacity: disabled ? 0.8 : 1,
        }}
        variant="surface"
        className="rt-Button"
        asChild
        role="button"
      >
        <button
          disabled={disabled}
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onClick();
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
            {historyItem.title}
          </Text>

          <Flex justify="between" style={{ marginTop: "8px" }}>
            <Text
              size="1"
              style={{ display: "flex", gap: "4px", alignItems: "center" }}
            >
              <ChatBubbleIcon />{" "}
              {historyItem.messages.filter(isUserMessage).length}
            </Text>

            <Text size="1">{dateTimeString}</Text>
          </Flex>
        </button>
      </Card>

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
              onOpenInTab(historyItem.id);
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
            onDelete(historyItem.id);
          }}
          iconSize={10}
          title="delete chat"
        />
      </Flex>
    </Box>
  );
};
