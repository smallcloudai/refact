import { Box, Flex, IconButton, Text } from "@radix-ui/themes";
import { ArchiveIcon, Cross2Icon } from "@radix-ui/react-icons";
import { useCallback, useEffect, useMemo, useState } from "react";
import classNames from "classnames";

import { clearPauseReasonsAndHandleToolsStatus } from "../../../features/ToolConfirmation/confirmationSlice";
import {
  useAppDispatch,
  useAppSelector,
  useCompressChat,
  useLastSentCompressionStop,
} from "../../../hooks";
import { popBackTo, push } from "../../../features/Pages/pagesSlice";
import { telemetryApi } from "../../../services/refact";
import {
  enableSend,
  newChatAction,
  selectChatId,
  setIsNewChatSuggestionRejected,
} from "../../../features/Chat";

import { Link } from "../../Link";

import styles from "./SuggestNewChat.module.css";
import { useUsageCounter } from "../../UsageCounter/useUsageCounter";

type SuggestNewChatProps = {
  shouldBeVisible?: boolean;
};

export const SuggestNewChat = ({
  shouldBeVisible = false,
}: SuggestNewChatProps) => {
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const { isWarning, isOverflown: isContextOverflown } = useUsageCounter();

  const [isRendered, setIsRendered] = useState(shouldBeVisible);
  const [isAnimating, setIsAnimating] = useState(false);
  const { compressChat, isCompressing } = useCompressChat();
  const lastSentCompression = useLastSentCompressionStop();

  useEffect(() => {
    if (shouldBeVisible) {
      setIsRendered(true);
      // small delay to ensure the initial state is rendered before animation
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          setIsAnimating(true);
        });
      });
    } else {
      setIsAnimating(false);
      const timer = setTimeout(() => {
        setIsRendered(false);
      }, 300);
      return () => {
        clearTimeout(timer);
      };
    }
  }, [shouldBeVisible]);

  const handleClose = () => {
    dispatch(setIsNewChatSuggestionRejected({ chatId, value: true }));
    dispatch(enableSend({ id: chatId }));

    void sendTelemetryEvent({
      scope: `dismissedNewChatSuggestionWarning`,
      success: true,
      error_message: "",
    });
  };

  const onCreateNewChat = useCallback(() => {
    const actions = [
      newChatAction(),
      clearPauseReasonsAndHandleToolsStatus({
        wasInteracted: false,
        confirmationStatus: true,
      }),
      popBackTo({ name: "history" }),
      push({ name: "chat" }),
    ];

    actions.forEach((action) => dispatch(action));
    void sendTelemetryEvent({
      scope: `openNewChat`,
      success: true,
      error_message: "",
    });
  }, [dispatch, sendTelemetryEvent]);

  const tipText = useMemo(() => {
    if (isWarning)
      return "This chat has been moderately compressed. The model may have limited access to earlier messages.";
    if (isContextOverflown)
      return "This chat has been heavily compressed. The model might not recall details from earlier conversations.";
    return "For best results, consider starting a new chat when switching topics.";
  }, [isWarning, isContextOverflown]);

  if (isCompressing) return null;

  return (
    <Box
      py="3"
      px="4"
      mb="1"
      flexShrink="0"
      display={isRendered ? "block" : "none"}
      className={classNames(styles.container, {
        [styles.visible]: isAnimating,
      })}
    >
      <Flex align="center" justify="between" gap="2" wrap="wrap">
        <Text size="1">
          <Text weight="bold">Tip:</Text> {tipText}
        </Text>

        <Flex align="center" mr="2" wrap="wrap" gap="2">
          <Link size="1" onClick={onCreateNewChat} color="indigo">
            Start a new chat
          </Link>
          {lastSentCompression.strength &&
            lastSentCompression.strength !== "absent" && (
              <Link
                size="1"
                onClick={() => {
                  void compressChat();
                }}
                color="indigo"
                asChild
              >
                <Flex
                  align="center"
                  justify="start"
                  gap="1"
                  display="inline-flex"
                >
                  <ArchiveIcon style={{ alignSelf: "start" }} />
                  Summarize and continue in a new chat.
                </Flex>
              </Link>
            )}
        </Flex>
        <Box position="absolute" top="1" right="1">
          <IconButton
            asChild
            variant="ghost"
            color="violet"
            size="1"
            onClick={handleClose}
          >
            <Cross2Icon />
          </IconButton>
        </Box>
      </Flex>
    </Box>
  );
};
