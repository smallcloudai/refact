import React from "react";
import { useGetMCPLogs } from "./useGetMCPLogs";
import { ScrollArea } from "../../ScrollArea";
import { Box, Flex, Heading, Text } from "@radix-ui/themes";
import { MarkdownCodeBlock } from "../../Markdown/CodeBlock";

type MCPLogsProps = {
  integrationPath: string;
  integrationName: string;
};

const formatMCPLogs = (logs: string[]): string => {
  return logs.join("\n");
};

export const MCPLogs: React.FC<MCPLogsProps> = ({
  integrationPath,
  integrationName,
}) => {
  const { data, isLoading } = useGetMCPLogs(integrationPath);

  if (!data) {
    if (isLoading) {
      return <div>Loading...</div>;
    }
    return <div>No data</div>;
  }

  const formattedData = formatMCPLogs(data.logs);

  return (
    <Flex direction="column" gap="4">
      <Heading as="h4" size="3">
        Runtime logs of {integrationName} server
      </Heading>
      <Text color="gray" size="2">
        Real-time diagnostic information from the MCP server. These logs help
        troubleshoot connection issues, monitor tool execution status, and
        verify proper server initialization. Critical for debugging when tools
        aren&apos;t appearing or functioning as expected.
      </Text>
      <ScrollArea scrollbars="horizontal" style={{ width: "100%" }} asChild>
        <Box maxHeight="250px">
          <MarkdownCodeBlock
            className="language-bash"
            startingLineNumber={1}
            showLineNumbers={true}
            preOptions={{
              noMargin: true,
            }}
          >
            {formattedData}
          </MarkdownCodeBlock>
        </Box>
      </ScrollArea>
    </Flex>
  );
};
