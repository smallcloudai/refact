import React, { useCallback } from "react";
import { Markdown } from "../Markdown";

import { Container, Box } from "@radix-ui/themes";
import { AssistantMessage, ToolCall } from "../../services/refact";
import { ToolContent } from "./ToolsContent";
import { fallbackCopying } from "../../utils/fallbackCopying";
import { telemetryApi } from "../../services/refact/telemetry";
import { ReasoningContent } from "./ReasoningContent";

type ChatInputProps = {
  reasoningContent?: string | null;
  toolCalls?: ToolCall[] | null;
  isLast?: boolean;
  children: AssistantMessage["ftm_content"];
};

export const AssistantInput: React.FC<ChatInputProps> = ({
  reasoningContent,
  toolCalls,
  children,
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
        <ReasoningContent
          reasoningContent={reasoningContent}
          onCopyClick={handleCopy}
        />
      )}
      {children && (
        <Box py="4">
          <Markdown canHaveInteractiveElements={true} onCopyClick={handleCopy}>
            {children}
          </Markdown>
        </Box>
      )}
      {toolCalls && <ToolContent toolCalls={toolCalls} />}
    </Container>
  );
};
