import React from "react";
import { Flex, Text, Box, Heading } from "@radix-ui/themes";
import type { FimDebugData } from "../../services/refact";
import { SearchContext } from "./SearchContext";
import { ScrollArea } from "../ScrollArea";

export type FimDebugProps = { data: FimDebugData };

export const FIMDebug: React.FC<FimDebugProps> = ({ data }) => {
  return (
    <ScrollArea scrollbars="vertical" fullHeight>
      {/** change scrollbars to both to remove word wrap */}
      <Flex direction="column" px="2" py="2" height="100%">
        <Heading size="4" wrap="nowrap" style={{ overflow: "hidden" }}>
          Code Completion Context
        </Heading>
        {data.context ? (
          <SearchContext data={data.context} />
        ) : (
          <Box py="2" overflow="hidden">
            <Text wrap="nowrap" size="2">
              Completion Context Cached response.
            </Text>
          </Box>
        )}

        <Box mt="auto" overflow="hidden">
          <Text wrap="nowrap" style={{ overflow: "hidden" }} size="1">
            {data.context?.fim_ms !== undefined && (
              <div>fim: {data.context.fim_ms}ms</div>
            )}
            {data.context?.rag_ms !== undefined && (
              <div>rag: {data.context.rag_ms}ms</div>
            )}
            {data.context?.n_ctx !== undefined && (
              <div>
                rag tokens: {data.context.rag_tokens_limit} /{" "}
                {data.context.n_ctx}
              </div>
            )}
            <div>model: {data.model}</div>
          </Text>
        </Box>
      </Flex>
    </ScrollArea>
  );
};
