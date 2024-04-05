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
    <ScrollArea scrollbars="both">
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
                  Cached
                </Text>
              </DataList.Label>
              <DataList.Value>{data.cached ? "true" : "false"}</DataList.Value>
            </DataList.Item>
            <DataList.Item>
              <DataList.Label>
                <Text size="1" weight="medium">
                  Snippet
                </Text>
              </DataList.Label>
              <DataList.Value>{data.snippet_telemetry_id}</DataList.Value>
            </DataList.Item>
            <DataList.Item>
              <DataList.Label>
                <Text size="1" weight="medium">
                  Model
                </Text>
              </DataList.Label>
              <DataList.Value>{data.model}</DataList.Value>
            </DataList.Item>
            {data.created && (
              <DataList.Item>
                <DataList.Label>
                  <Text size="1" weight="medium">
                    Created
                  </Text>
                </DataList.Label>
                <DataList.Value>{data.created}</DataList.Value>
              </DataList.Item>
            )}
            {data.elapsed && (
              <DataList.Item>
                <DataList.Label>
                  <Text size="1" weight="medium">
                    Elapsed
                  </Text>
                </DataList.Label>
                <DataList.Value>{data.elapsed}</DataList.Value>
              </DataList.Item>
            )}
          </DataList.Root>
        </Section>

        {/* <Heading size="5">Choices</Heading>
      <Section size="1">
        {data.choices.map((choice, i) => {
          return (
            <DataList.Root
              key={i}
              orientation={{
                initial: "vertical",
                xs: "horizontal",
              }}
            >
              <DataList.Item>
                <DataList.Label>Index</DataList.Label>
                <DataList.Value>{choice.index}</DataList.Value>
              </DataList.Item>
              <DataList.Item>
                <DataList.Label>Code</DataList.Label>
                <DataList.Value>
                  <Code>{choice.code_completion}</Code>
                </DataList.Value>
              </DataList.Item>
              <DataList.Item>
                <DataList.Label>Finish reason</DataList.Label>
                <DataList.Value>{choice.finish_reason}</DataList.Value>
              </DataList.Item>
            </DataList.Root>
          );
        })}
      </Section> */}

        {data.context && <SearchContext data={data.context} />}
      </Flex>
    </ScrollArea>
  );
};
