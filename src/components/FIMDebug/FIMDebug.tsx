import React from "react";
import { Flex, Text, Box } from "@radix-ui/themes";
import type { FimDebugData } from "../../services/refact";
import { SearchContext } from "./SearchContext";
import { ScrollArea } from "../ScrollArea";

export type FimDebugProps = { data: FimDebugData };

export const FIMDebug: React.FC<FimDebugProps> = ({ data }) => {
  return (
    <ScrollArea scrollbars="vertical" fullHeight>
      {/** change scrollbars to both to remove word wrap */}
      <Flex direction="column" px="2" height="100%">
        {data.context && <SearchContext data={data.context} />}

        <Box mt="auto" pb="2">
          <Text wrap="nowrap" style={{ overflow: "hidden" }} size="1">
            model: {data.model}
          </Text>
        </Box>
      </Flex>
    </ScrollArea>
  );
};
