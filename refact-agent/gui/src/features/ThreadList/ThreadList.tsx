import React, { useCallback, useEffect } from "react";
import { Box, Flex, Text } from "@radix-ui/themes";
import { useAppSelector, useAppDispatch } from "../../hooks";
import {
  selectThreadList,
  selectThreadListError,
  selectThreadListLoading,
  ThreadListItem,
} from "./threadListSlice";
import {
  graphqlQueriesAndMutations,
  threadsPageSub,
} from "../../services/graphql";
import { selectActiveGroup } from "../../features/Teams/teamsSlice";
import { ScrollArea } from "../../components/ScrollArea";
import { ChatBubbleIcon } from "@radix-ui/react-icons";
import { CloseButton } from "../../components/Buttons/Buttons";
import { CardButton } from "../../components/Buttons";

import { pagesSlice } from "../Pages/pagesSlice";

function useThreadPageSub() {
  const dispatch = useAppDispatch();

  const activeProject = useAppSelector(selectActiveGroup);
  const loading = useAppSelector(selectThreadListLoading);
  const error = useAppSelector(selectThreadListError);
  const threads = useAppSelector(selectThreadList);

  const onOpen = useCallback(
    (ft_id: string) => {
      dispatch(pagesSlice.actions.push({ name: "chat", ft_id }));
    },
    [dispatch],
  );

  useEffect(() => {
    if (activeProject === null) return;
    const thunk = dispatch(
      threadsPageSub({
        located_fgroup_id: activeProject.id,
        limit: 100,
      }),
    );

    return () => {
      thunk.abort("unmounted");
    };
  }, [activeProject, dispatch]);

  return {
    loading,
    error,
    threads,
    onOpen,
  };
}

export const ThreadList: React.FC = () => {
  // TODO: error and loading states
  const { threads, onOpen } = useThreadPageSub();

  return (
    <Box
      style={{
        overflow: "hidden",
      }}
      pb="2"
      flexGrow="1"
    >
      <ScrollArea scrollbars="vertical">
        <Flex
          justify="center"
          align={threads.length > 0 ? "center" : "start"}
          //   pl="2"
          //   pr="2"
          p="2"
          direction="column"
          gap="1"
        >
          {threads.map((thread) => (
            <ThreadListItem
              key={thread.ft_id}
              thread={thread}
              onOpen={onOpen}
            />
          ))}
          {/* {sortedHistory.length !== 0 ? (
              sortedHistory.map((item) => (
                <HistoryItem
                  onClick={() => onHistoryItemClick(item)}
                  onOpenInTab={onOpenChatInTab}
                  onDelete={onDeleteHistoryItem}
                  key={item.id}
                  historyItem={item}
                  disabled={item.id === currentChatId}
                />
              ))
            ) : (
              <Text as="p" size="2" mt="2">
                Your chat history is currently empty. Click &quot;New Chat&quot;
                to start a conversation.
              </Text>
            )} */}
        </Flex>
      </ScrollArea>
    </Box>
  );
};

type ThreadItemProps = {
  thread: ThreadListItem;
  onOpen: (id: string) => void;
};

const ThreadListItem: React.FC<ThreadItemProps> = ({ thread, onOpen }) => {
  // TODO: handel updating state
  // TODO: handle read state
  // TODO: change this to created at

  const dateCreated = new Date(thread.ft_created_ts * 1000);
  const dateTimeString = dateCreated.toLocaleString();
  const [deleteThread, deleteThreadRequest] =
    graphqlQueriesAndMutations.useDeleteThreadMutation();
  return (
    <Box style={{ position: "relative", width: "100%" }}>
      <CardButton
        //   disabled={disabled}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          onOpen(thread.ft_id);
        }}
      >
        <Flex gap="2px" align="center">
          {/* {isStreaming && <Spinner style={{ minWidth: 16, minHeight: 16 }} />} */}
          {/* {thread.ft_anything_new && (
            <DotFilledIcon style={{ minWidth: 16, minHeight: 16 }} />
          )} */}
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
            {thread.ft_title}
          </Text>
        </Flex>

        <Flex justify="between" mt="8px">
          <Flex gap="4">
            <Text
              size="1"
              style={{ display: "flex", gap: "4px", alignItems: "center" }}
            >
              <ChatBubbleIcon />{" "}
              {/* {historyItem.messages.filter(isUserMessage).length} */}
            </Text>
            {/** TODO: total cost */}
            {/* {totalCost ? (
                    <Text
                      size="1"
                      style={{ display: "flex", gap: "4px", alignItems: "center" }}
                    >
                      <Coin width="15px" height="15px" /> {Math.round(totalCost)}
                    </Text>
                  ) : (
                    false
                  )} */}
          </Flex>

          <Text size="1">{dateTimeString}</Text>
        </Flex>
      </CardButton>
      <Flex
        position="absolute"
        top="6px"
        right="6px"
        gap="1"
        justify="end"
        align="center"
      >
        <CloseButton
          loading={deleteThreadRequest.isLoading}
          size="1"
          onClick={(event) => {
            event.preventDefault();
            event.stopPropagation();
            void deleteThread({ id: thread.ft_id });
          }}
          iconSize={10}
          title="delete thread"
        />
      </Flex>
    </Box>
  );
};
