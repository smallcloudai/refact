import { Box, Flex, IconButton, Text } from "@radix-ui/themes";
import { Cross2Icon } from "@radix-ui/react-icons";
import { useCallback, useEffect, useMemo, useState } from "react";

import { clearPauseReasonsAndHandleToolsStatus } from "../../../features/ToolConfirmation/confirmationSlice";
import { useAppDispatch, useAppSelector } from "../../../hooks";
import { popBackTo, push } from "../../../features/Pages/pagesSlice";
import { telemetryApi } from "../../../services/refact";
import {
  newChatAction,
  selectChatId,
  selectThreadNewChatSuggested,
  setIsNewChatSuggestionRejected,
} from "../../../features/Chat";

import { Link } from "../../Link";

import styles from "./SuggestNewChat.module.css";
import classNames from "classnames";
import { useUsageCounter } from "../../UsageCounter/useUsageCounter";

type SuggestNewChatProps = {
  shouldBeVisible?: boolean;
};

export const SuggestNewChat = ({
  shouldBeVisible = false,
}: SuggestNewChatProps) => {
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const { isMandatory } = useAppSelector(selectThreadNewChatSuggested);
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const { isWarning, isOverflown: isContextOverflown } = useUsageCounter();

  const [isRendered, setIsRendered] = useState(shouldBeVisible);
  const [isAnimating, setIsAnimating] = useState(false);

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
      return "Long chats cause you to reach your usage limits faster.";
    if (isContextOverflown)
      return "Maximum available context for this chat is exceeded. Consider starting a new chat.";
    return "Agent performs better for chats which don't switch topics often.";
  }, [isWarning, isContextOverflown]);

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
      <Flex align="center" justify="between" gap="2">
        <Text size="1">
          <Text weight="bold">Tip:</Text> {tipText}
        </Text>
        <Flex align="center" gap="3" flexShrink="0">
          <Link size="1" onClick={onCreateNewChat} color="indigo">
            Start a new chat
          </Link>
          {!isMandatory && (
            <IconButton
              asChild
              variant="ghost"
              color="violet"
              size="1"
              onClick={handleClose}
            >
              <Cross2Icon />
            </IconButton>
          )}
        </Flex>
      </Flex>
    </Box>
  );
};
