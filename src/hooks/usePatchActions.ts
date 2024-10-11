import { useState, useCallback, useMemo } from "react";
import { useSelector } from "react-redux";
import {
  AssistantMessage,
  diffApi,
  isAssistantMessage,
  isDetailMessage,
} from "../services/refact";
import {
  selectMessages,
  selectIsStreaming,
  selectIsWaiting,
} from "../features/Chat";
import { useEventsBusForIDE } from "./useEventBusForIDE";

export const usePatchActions = () => {
  const {
    diffPreview,
    startFileAnimation,
    stopFileAnimation,
    openFile,
    writeResultsToFile,
  } = useEventsBusForIDE();
  const messages = useSelector(selectMessages);
  const isStreaming = useSelector(selectIsStreaming);
  const isWaiting = useSelector(selectIsWaiting);

  const [errorMessage, setErrorMessage] = useState<{
    type: "warning" | "error";
    text: string;
  } | null>(null);

  const resetErrorMessage = useCallback(() => {
    setErrorMessage(null);
  }, []);

  const [getPatch, patchResult] =
    diffApi.usePatchSingleFileFromTicketMutation();

  const disable = useMemo(() => {
    return !!errorMessage || isStreaming || isWaiting || patchResult.isLoading;
  }, [errorMessage, isStreaming, isWaiting, patchResult.isLoading]);

  const pinMessages = useMemo(() => {
    const assistantMessages: AssistantMessage[] =
      messages.filter(isAssistantMessage);

    const lines = assistantMessages.reduce<string[]>((acc, curr) => {
      if (!curr.content) return acc;
      return acc.concat(curr.content.split("\n"));
    }, []);

    return lines.filter((line) => line.startsWith("📍"));
  }, [messages]);

  const handleShow = useCallback(
    (pin: string) => {
      const [, , fileName] = pin.split(" ");
      startFileAnimation(fileName);
      getPatch({ pin, messages })
        .unwrap()
        .then((maybeDetail) => {
          if (isDetailMessage(maybeDetail)) {
            const error = new Error(maybeDetail.detail);
            throw error;
          }
          return maybeDetail;
        })
        .then((patch) => {
          stopFileAnimation(fileName);
          diffPreview(patch, pin, pinMessages);
        })
        .catch((error: Error | { data: { detail: string } }) => {
          stopFileAnimation(fileName);
          if ("message" in error) {
            setErrorMessage({
              type: "error",
              text: "Failed to open patch: " + error.message,
            });
          } else {
            setErrorMessage({
              type: "error",
              text: "Failed to open patch: " + error.data.detail,
            });
          }
        });
    },
    [
      diffPreview,
      getPatch,
      messages,
      pinMessages,
      startFileAnimation,
      stopFileAnimation,
    ],
  );

  const handleApply = useCallback(
    (pin: string) => {
      const [, , fileName] = pin.split(" ");
      startFileAnimation(fileName);

      getPatch({ pin, messages })
        .unwrap()
        .then((maybeDetail) => {
          if (isDetailMessage(maybeDetail)) {
            const error = new Error(maybeDetail.detail);
            throw error;
          }
          return maybeDetail;
        })
        .then((patch) => {
          stopFileAnimation(fileName);
          writeResultsToFile(patch.results);
        })
        .catch((error: Error | { data: { detail: string } }) => {
          stopFileAnimation(fileName);
          if ("message" in error) {
            setErrorMessage({
              type: "error",
              text: "Failed to apply patch: " + error.message,
            });
          } else {
            setErrorMessage({
              type: "error",
              text: "Failed to apply patch: " + error.data.detail,
            });
          }
        });
    },
    [
      getPatch,
      messages,
      startFileAnimation,
      stopFileAnimation,
      writeResultsToFile,
    ],
  );

  return {
    errorMessage,
    handleShow,
    patchResult,
    handleApply,
    resetErrorMessage,
    disable,
    openFile,
  };
};
