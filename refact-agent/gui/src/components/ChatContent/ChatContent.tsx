import React, { useCallback, useMemo } from "react";

import { ScrollArea, ScrollAreaWithAnchor } from "../ScrollArea";
import { Flex, Container, Button, Box } from "@radix-ui/themes";
import styles from "./ChatContent.module.css";

import { useAppDispatch, useDiffFileReload } from "../../hooks";
import { useAppSelector } from "../../hooks";
import {
  selectIntegrationMeta,
  selectIsStreaming,
  selectIsThreadRunning,
  selectIsWaiting,
  selectThreadId,
  selectToolConfirmationRequests,
} from "../../features/ThreadMessages";

import { ChatLinks } from "../ChatLinks";
import { PlaceHolderText } from "./PlaceHolderText";
import { UsageCounter } from "../UsageCounter";
import { useUsageCounter } from "../UsageCounter/useUsageCounter.ts";
import { LogoAnimation } from "../LogoAnimation/LogoAnimation.tsx";
import { selectThreadMessageTrie } from "../../features/ThreadMessages";
import { MessageNode } from "../MessageNode/MessageNode.tsx";
import { isEmptyNode } from "../../features/ThreadMessages/makeMessageTrie.ts";
import { pauseThreadThunk } from "../../services/graphql/graphqlThunks.ts";
import { popBackTo } from "../../features/Pages/pagesSlice.ts";

const usePauseThread = () => {
  const dispatch = useAppDispatch();
  const isThreadRunning = useAppSelector(selectIsThreadRunning);
  const threadId = useAppSelector(selectThreadId);
  const toolConfirmationRequests = useAppSelector(
    selectToolConfirmationRequests,
  );

  const shouldShowStopButton = useMemo(() => {
    if (!threadId) return false;
    if (toolConfirmationRequests.length > 0) return false;
    // if (pauseReasonsWithPause.pause) return false;
    return isThreadRunning;
  }, [threadId, toolConfirmationRequests.length, isThreadRunning]);

  const handlePause = useCallback(() => {
    if (!threadId) return;
    void dispatch(pauseThreadThunk({ id: threadId }));
  }, [dispatch, threadId]);

  return { shouldShowStopButton, handlePause };
};

export const ChatContent: React.FC = () => {
  const dispatch = useAppDispatch();
  // TODO: stays when creating a new chat :/
  const threadMessageTrie = useAppSelector(selectThreadMessageTrie);
  const isStreaming = useAppSelector(selectIsStreaming);

  const { shouldShow } = useUsageCounter();
  const isWaiting = useAppSelector(selectIsWaiting);

  const integrationMeta = useAppSelector(selectIntegrationMeta);
  const toolConfirmationRequests = useAppSelector(
    selectToolConfirmationRequests,
  );

  const { shouldShowStopButton, handlePause } = usePauseThread();

  const handleReturnToConfigurationClick = useCallback(() => {
    // console.log(`[DEBUG]: going back to configuration page`);
    // TBD: should it be allowed to run in the background?
    dispatch(
      popBackTo({
        name: "integrations page",
        projectPath: integrationMeta?.project,
        integrationName: integrationMeta?.name,
        integrationPath: integrationMeta?.path,
        wasOpenedThroughChat: true,
      }),
    );
  }, [
    dispatch,
    integrationMeta?.name,
    integrationMeta?.path,
    integrationMeta?.project,
  ]);

  const shouldConfigButtonBeVisible = useMemo(() => {
    return !integrationMeta?.path?.includes("project_summary");
  }, [integrationMeta?.path]);

  // Dedicated hook for handling file reloads
  useDiffFileReload();

  return (
    <ScrollAreaWithAnchor.ScrollArea
      style={{ flexGrow: 1, height: "auto", position: "relative" }}
      scrollbars="vertical"
      type={isWaiting || isStreaming ? "auto" : "hover"}
      fullHeight
    >
      <Flex
        direction="column"
        className={styles.content}
        data-element="ChatContent"
        p="2"
        gap="1"
      >
        {isEmptyNode(threadMessageTrie) ? (
          <Container>
            <PlaceHolderText />
          </Container>
        ) : (
          <MessageNode>{threadMessageTrie}</MessageNode>
        )}
        {/* {renderMessages(messages, onRetryWrapper, isWaiting)} */}
        {/* <Container>
          <UncommittedChangesWarning />
        </Container> */}
        {shouldShow && <UsageCounter />}
        <Container pt="4" pb="8">
          {toolConfirmationRequests.length === 0 && (
            <LogoAnimation
              size="8"
              isStreaming={isStreaming}
              isWaiting={isWaiting}
            />
          )}
        </Container>
      </Flex>

      <Box
        style={{
          position: "absolute",
          bottom: 0,
          maxWidth: "100%", // TODO: make space for the down button
        }}
      >
        <ScrollArea scrollbars="horizontal">
          <Flex align="start" gap="3" pb="2">
            {shouldShowStopButton && (
              <Button
                // ml="auto"
                color="red"
                title="Pause thread"
                onClick={handlePause}
              >
                Stop
              </Button>
            )}
            {shouldConfigButtonBeVisible && (
              <Button
                // ml="auto"
                color="gray"
                title="Return to configuration page"
                onClick={handleReturnToConfigurationClick}
              >
                Return
              </Button>
            )}

            <ChatLinks />
          </Flex>
        </ScrollArea>
      </Box>
    </ScrollAreaWithAnchor.ScrollArea>
  );
};

ChatContent.displayName = "ChatContent";
