import React from "react";
import {
  Flex,
  Section,
  Heading,
  // Code,
  DataList,
} from "@radix-ui/themes";
import type { FimDebugData } from "../../services/refact";
import { SearchContext } from "./SearchContext";

export type FimDebugProps = { data: FimDebugData };

export const FIMDebug: React.FC<FimDebugProps> = ({ data }) => {
  return (
    <Flex direction="column">
      <Heading>FIM debug</Heading>
      <Section size="1">
        <DataList.Root
          style={{
            gap: "var(--space-2)",
          }}
          orientation={{
            initial: "vertical",
            xs: "horizontal",
          }}
        >
          <DataList.Item>
            <DataList.Label>Cached</DataList.Label>
            <DataList.Value>{data.cached ? "true" : "false"}</DataList.Value>
          </DataList.Item>
          <DataList.Item>
            <DataList.Label>Snippet</DataList.Label>
            <DataList.Value>{data.snippet_telemetry_id}</DataList.Value>
          </DataList.Item>
          <DataList.Item>
            <DataList.Label>Model</DataList.Label>
            <DataList.Value>{data.model}</DataList.Value>
          </DataList.Item>
          <DataList.Item>
            <DataList.Label>Created</DataList.Label>
            <DataList.Value>{data.created}</DataList.Value>
          </DataList.Item>
          <DataList.Item>
            <DataList.Label>Elapsed</DataList.Label>
            <DataList.Value>{data.elapsed}</DataList.Value>
          </DataList.Item>
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

      <Heading size="5" mb="1">
        Search Context
      </Heading>
      {/** TODO: figure out if context is an array or an object */}
      {data.context && <SearchContext data={data.context} />}
    </Flex>
  );
};
