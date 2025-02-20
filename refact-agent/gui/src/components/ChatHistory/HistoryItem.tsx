import React from "react";
import { Card, Flex, Text, Box } from "@radix-ui/themes";
import { ChatBubbleIcon, DotFilledIcon } from "@radix-ui/react-icons";
import { CloseButton } from "../Buttons/Buttons";
import { CThread } from "../../services/refact";

export const HistoryItem: React.FC<{
  historyItem: CThread;
  onClick: (thread: CThread) => void;
  onDelete: (id: string) => void;
  // onOpenInTab?: (id: string) => void;
  // disabled: boolean;
}> = ({
  historyItem,
  onClick,
  onDelete,
  // onOpenInTab,
  // disabled
}) => {
  const dateCreated = new Date(historyItem.cthread_created_ts);
  const dateTimeString = dateCreated.toLocaleString();
  // maybe remove this?
  // const cache = useAppSelector((app) => app.chat.cache);
  // const isStreaming = historyItem.cthread_id in cache;
  return (
    <Box style={{ position: "relative", width: "100%" }}>
      <Card
        style={{
          width: "100%",
          marginBottom: "2px",
          // opacity: disabled ? 0.8 : 1,
        }}
        variant="surface"
        className="rt-Button"
        asChild
        role="button"
      >
        <button
          // disabled={disabled}
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onClick(historyItem);
          }}
        >
          <Flex gap="2px" align="center">
            {/* {isStreaming && <Spinner style={{ minWidth: 16, minHeight: 16 }} />}
            {!isStreaming && historyItem.cthread_anything_new && (
              <DotFilledIcon style={{ minWidth: 16, minHeight: 16 }} />
            )} */}
            {historyItem.cthread_anything_new && (
              <DotFilledIcon style={{ minWidth: 16, minHeight: 16 }} />
            )}
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
              {historyItem.cthread_title}
            </Text>
          </Flex>

          <Flex justify="between" mt="8px">
            <Text
              size="1"
              style={{ display: "flex", gap: "4px", alignItems: "center" }}
            >
              <ChatBubbleIcon /> {historyItem.cthread_n}
            </Text>

            <Text size="1">{dateTimeString}</Text>
          </Flex>
        </button>
      </Card>

      <Flex
        position="absolute"
        top="6px"
        right="6px"
        gap="1"
        justify="end"
        align="center"
        // justify to flex end
      >
        {/**TODO: open in tab button */}
        {/* {onOpenInTab && (
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
        )} */}

        <CloseButton
          size="1"
          // needs to be smaller
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            onDelete(historyItem.cthread_id);
          }}
          iconSize={10}
          title="delete chat"
        />
      </Flex>
    </Box>
  );
};
