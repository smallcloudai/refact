import React, { useCallback } from "react";
import { Markdown } from "../Markdown";

import { Container, Box, Flex } from "@radix-ui/themes";
import { ToolCall, Usage } from "../../services/refact";
import { ToolContent } from "./ToolsContent";
import { fallbackCopying } from "../../utils/fallbackCopying";
import { telemetryApi } from "../../services/refact/telemetry";
import { LikeButton } from "./LikeButton";
import { ResendButton } from "./ResendButton";
import { ReasoningContent } from "./ReasoningContent";
import { MessageUsageInfo } from "./MessageUsageInfo";

type ChatInputProps = {
  message: string | null;
  reasoningContent?: string | null;
  toolCalls?: ToolCall[] | null;
  isLast?: boolean;
  usage?: Usage | null;
  metering_coins_prompt?: number;
  metering_coins_generated?: number;
  metering_coins_cache_creation?: number;
  metering_coins_cache_read?: number;
};

export const AssistantInput: React.FC<ChatInputProps> = ({
  message,
  reasoningContent,
  toolCalls,
  isLast,
  usage,
  metering_coins_prompt,
  metering_coins_generated,
  metering_coins_cache_creation,
  metering_coins_cache_read,
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

  const hasMessageFirst = !reasoningContent && message;

  return (
    <Container position="relative">
      <MessageUsageInfo
        usage={usage}
        metering_coins_prompt={metering_coins_prompt}
        metering_coins_generated={metering_coins_generated}
        metering_coins_cache_creation={metering_coins_cache_creation}
        metering_coins_cache_read={metering_coins_cache_read}
        topOffset={hasMessageFirst ? "var(--space-4)" : "0"}
      />
      {reasoningContent && (
        <ReasoningContent
          reasoningContent={reasoningContent}
          onCopyClick={handleCopy}
        />
      )}
      {message && (
        <Box py="4">
          <Markdown canHaveInteractiveElements={true} onCopyClick={handleCopy}>
            {message}
          </Markdown>
        </Box>
      )}
      {toolCalls && <ToolContent toolCalls={toolCalls} />}
      {isLast && (
        <Flex justify="end" px="2" gap="2" align="center">
          <ResendButton />
          <LikeButton />
        </Flex>
      )}
    </Container>
  );
};
