import React from "react";
import {
  Flex,
  Text,
  Section,
  Heading,
  Container,
  Code,
  DataList,
  Table,
} from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import type { FimDebugData } from "../../services/refact";

export type FimDebugProps = { data: FimDebugData };

export const FIMDebug: React.FC<FimDebugProps> = ({ data }) => {
  return (
    <Flex direction="column">
      <Heading>FIM debug</Heading>
      <Section size="1">
        <DataList.Root
          orientation={{
            initial: "vertical",
            xs: "horizontal",
          }}
        >
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

      <Heading size="5">Choices</Heading>
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
      </Section>

      <Heading size="5">Search Context</Heading>

      <Section size="1">
        <Table.Root variant="surface">
          <Table.Header>
            <Table.Row>
              <Table.ColumnHeaderCell>Symbol</Table.ColumnHeaderCell>
              <Table.ColumnHeaderCell>From</Table.ColumnHeaderCell>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {data.context.was_looking_for.map((item, index) => {
              return (
                <Table.Row key={index}>
                  <Table.RowHeaderCell>{item.symbol}</Table.RowHeaderCell>
                  <Table.Cell>{item.from}</Table.Cell>
                </Table.Row>
              );
            })}
          </Table.Body>
        </Table.Root>

        <Section size="1">
          {data.context.attached_files.map((file, i) => {
            return (
              <Container key={i}>
                <Text>
                  File: {file.file_name}:{file.line1}-${file.line2}
                </Text>
                <Markdown>{"```\n" + file.file_content + "\n```"}</Markdown>
              </Container>
            );
          })}
        </Section>
      </Section>
    </Flex>
  );
};
