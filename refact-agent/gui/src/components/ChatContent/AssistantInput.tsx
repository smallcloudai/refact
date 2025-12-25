import React, { useCallback, useMemo } from "react";
import { Markdown } from "../Markdown";

import { Container, Box, Flex, Text, Link, Card } from "@radix-ui/themes";
import { ThinkingBlock, ToolCall, WebSearchCitation } from "../../services/refact";
import { ToolContent } from "./ToolsContent";
import { fallbackCopying } from "../../utils/fallbackCopying";
import { telemetryApi } from "../../services/refact/telemetry";
import { ReasoningContent } from "./ReasoningContent";

type ChatInputProps = {
  message: string | null;
  reasoningContent?: string | null;
  thinkingBlocks?: ThinkingBlock[] | null;
  toolCalls?: ToolCall[] | null;
  serverExecutedTools?: ToolCall[] | null;
  citations?: WebSearchCitation[] | null;
};

export const AssistantInput: React.FC<ChatInputProps> = ({
  message,
  reasoningContent,
  thinkingBlocks,
  toolCalls,
  serverExecutedTools,
  citations,
}) => {
  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  // Get unique server-executed tool names for display
  const serverToolNames = useMemo(() => {
    if (!serverExecutedTools || serverExecutedTools.length === 0) return [];
    const names = serverExecutedTools
      .map((tool) => tool.function.name)
      .filter((name): name is string => !!name);
    return [...new Set(names)];
  }, [serverExecutedTools]);

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

  // Combine reasoning_content and thinking_blocks into one display
  const combinedReasoning = useMemo(() => {
    const parts: string[] = [];
    if (reasoningContent) {
      parts.push(reasoningContent);
    }
    if (thinkingBlocks && thinkingBlocks.length > 0) {
      const thinkingText = thinkingBlocks
        .filter((block) => block.thinking)
        .map((block) => block.thinking)
        .join("\n\n");
      if (thinkingText) {
        parts.push(thinkingText);
      }
    }
    return parts.length > 0 ? parts.join("\n\n") : null;
  }, [reasoningContent, thinkingBlocks]);

  return (
    <Container position="relative">
      {combinedReasoning && (
        <ReasoningContent
          reasoningContent={combinedReasoning}
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
      {/* Server-executed tools indicator with citations */}
      {(serverToolNames.length > 0 || (citations && citations.length > 0)) && (
        <Card my="3" style={{ backgroundColor: "var(--gray-a2)" }}>
          <Flex direction="column" gap="2" p="2">
            {serverToolNames.length > 0 && (
              <Flex gap="2" align="center">
                <Text size="2">☁️</Text>
                <Text size="2" color="gray">
                  {serverToolNames.join(", ")}
                </Text>
              </Flex>
            )}
            {citations && citations.length > 0 && (
              <Flex
                direction="column"
                gap="1"
                style={{ maxHeight: "150px", overflowY: "auto" }}
              >
                <Text size="1" weight="medium" color="gray">
                  Sources:
                </Text>
                {citations
                  .filter(
                    (citation, idx, arr) =>
                      arr.findIndex((c) => c.url === citation.url) === idx,
                  )
                  .map((citation, idx) => (
                    <Link
                      key={idx}
                      href={citation.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      size="1"
                    >
                      {citation.title}
                    </Link>
                  ))}
              </Flex>
            )}
          </Flex>
        </Card>
      )}
      {toolCalls && <ToolContent toolCalls={toolCalls} />}
    </Container>
  );
};
