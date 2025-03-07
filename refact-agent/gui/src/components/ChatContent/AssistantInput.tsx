import React, { useCallback } from "react";
import { Markdown } from "../Markdown";

import { Container, Box } from "@radix-ui/themes";
import { ToolCall } from "../../services/refact";
import { ToolContent } from "./ToolsContent";
import { fallbackCopying } from "../../utils/fallbackCopying";
import { telemetryApi } from "../../services/refact/telemetry";
import { LikeButton } from "./LikeButton";
import styles from "./ReasoningContent.module.css";

type ChatInputProps = {
  message: string | null;
  reasoningContent?: string | null;
  toolCalls?: ToolCall[] | null;
  isLast?: boolean;
};

export const AssistantInput: React.FC<ChatInputProps> = ({
  message,
  reasoningContent,
  toolCalls,
  isLast,
}) => {
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const handleCopy = useCallback(
    (text: string) => {
      // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition
      if (window.navigator?.clipboard?.writeText) {
        void window.navigator.clipboard
          .writeText(text)
          .catch(() => {
            // eslint-disable-next-line no-console
            console.log("failed to copy to clipboard");
            void sendTelemetryEvent({
              scope: `codeBlockCopyToClipboard`,
              success: false,
              error_message:
                "window.navigator?.clipboard?.writeText: failed to copy to clipboard",
            });
          })
          .then(() => {
            void sendTelemetryEvent({
              scope: `codeBlockCopyToClipboard`,
              success: true,
              error_message: "",
            });
          });
      } else {
        fallbackCopying(text);
        void sendTelemetryEvent({
          scope: `codeBlockCopyToClipboard`,
          success: true,
          error_message: "",
        });
      }
    },
    [sendTelemetryEvent],
  );

  return (
    <Container position="relative">
      {reasoningContent && (
        <Box py="2">
          <div className={styles.reasoningCallout}>
            <div className={styles.reasoningTitle}>Model Reasoning</div>
            <div className={styles.reasoningContent}>
              <Markdown
                canHaveInteractiveElements={true}
                onCopyClick={handleCopy}
              >
                {reasoningContent}
              </Markdown>
            </div>
          </div>
        </Box>
      )}
      {message && (
        <Box py="4">
          <Markdown canHaveInteractiveElements={true} onCopyClick={handleCopy}>
            {message}
          </Markdown>
        </Box>
      )}
      {toolCalls && <ToolContent toolCalls={toolCalls} />}
      {isLast && <LikeButton />}
    </Container>
  );
};
