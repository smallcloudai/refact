import React, { useCallback, useMemo } from "react";
import { Markdown } from "../Markdown";

import { Container, Box, Flex, Text, Link, Card } from "@radix-ui/themes";
import { ToolCall, Usage, WebSearchCitation } from "../../services/refact";
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
  serverExecutedTools?: ToolCall[] | null; // Tools that were executed by the provider (srvtoolu_*)
  citations?: WebSearchCitation[] | null;
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
  serverExecutedTools,
  citations,
  isLast,
  usage,
  metering_coins_prompt,
  metering_coins_generated,
  metering_coins_cache_creation,
  metering_coins_cache_read,
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
        <Box py="4" style={{ paddingRight: "50px" }}>
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
      {isLast && (
        <Flex justify="end" px="2" py="2" gap="2" align="center" pr="4">
          <ResendButton />
          <LikeButton />
        </Flex>
      )}
    </Container>
  );
};
