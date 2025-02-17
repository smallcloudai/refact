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
  selectActiveFile,
  selectSelectedSnippet,
} from "../features/Chat";
import { useEventsBusForIDE } from "./useEventBusForIDE";
import { useAppSelector } from "./useAppSelector";
import { extractFilePathFromPin } from "../utils";

export const usePatchActions = () => {
  const {
    diffPreview,
    startFileAnimation,
    stopFileAnimation,
    openFile,
    writeResultsToFile,
    diffPasteBack,
  } = useEventsBusForIDE();
  const messages = useSelector(selectMessages);
  const isStreaming = useSelector(selectIsStreaming);
  const isWaiting = useSelector(selectIsWaiting);

  const activeFile = useAppSelector(selectActiveFile);

  const snippet = useAppSelector(selectSelectedSnippet);

  const codeLineCount = useMemo(() => {
    if (snippet.code.length === 0) return 0;
    return snippet.code.split("\n").filter((str) => str).length;
  }, [snippet.code]);

  const canPaste = useMemo(
    () => activeFile.can_paste && codeLineCount > 0,
    [activeFile.can_paste, codeLineCount],
  );

  const [errorMessage, setErrorMessage] = useState<{
    type: "warning" | "error";
    text: string;
  } | null>(null);

  const resetErrorMessage = useCallback(() => {
    setErrorMessage(null);
  }, []);

  const [getPatch, patchResult] = diffApi.usePatchSingleFileFromTicketMutation(
    {},
  );

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
      const fileName = extractFilePathFromPin(pin);
      const cleanedFileName = fileName.replace(/\\\?\\|^\\+/g, "");
      startFileAnimation(cleanedFileName);
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
          stopFileAnimation(cleanedFileName);

          if (patch.results.every((result) => result.already_applied)) {
            const errorText =
              "Already applied, no significant changes generated.";
            setErrorMessage({
              type: "warning",
              text: errorText,
            });
          } else {
            diffPreview(patch, pin, pinMessages);
          }
        })
        .catch((error: Error | { data: { detail: string } }) => {
          stopFileAnimation(cleanedFileName);
          let text = "";
          if ("message" in error) {
            text = "Failed to open patch: " + error.message;
          } else {
            text = "Failed to open patch: " + error.data.detail;
          }
          setErrorMessage({
            type: "error",
            text: text,
          });
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
      const fileName = extractFilePathFromPin(pin);
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
          if (patch.results.every((result) => result.already_applied)) {
            setErrorMessage({
              type: "warning",
              text: "Already applied, no significant changes generated.",
            });
          } else {
            writeResultsToFile(patch.results);
          }
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

    handlePaste: diffPasteBack,
    canPaste,
  };
};
