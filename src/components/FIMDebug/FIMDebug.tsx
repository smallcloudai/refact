import React from "react";
import {
  Flex,
  Section,
  Heading,
  // Code,
  DataList,
  Text,
} from "@radix-ui/themes";
import type { FimDebugData } from "../../services/refact";
import { SearchContext } from "./SearchContext";
import { ScrollArea } from "../ScrollArea";

export type FimDebugProps = { data: FimDebugData };

export const FIMDebug: React.FC<FimDebugProps> = ({ data }) => {
  return (
    <ScrollArea scrollbars="vertical">
      <Flex direction="column">
        <Heading size="4">FIM debug</Heading>
        <Section size="1" py="4">
          <DataList.Root
            trim="both"
            style={{
              gap: "var(--space-2)",
            }}
            orientation={{
              initial: "vertical",
              xs: "horizontal",
            }}
          >
            <DataList.Item>
              <DataList.Label>
                <Text size="1" weight="medium">
                  Model
                </Text>
              </DataList.Label>
              <DataList.Value>{data.model}</DataList.Value>
            </DataList.Item>
          </DataList.Root>
        </Section>

        {data.context && <SearchContext data={data.context} />}
      </Flex>
    </ScrollArea>
  );
};
