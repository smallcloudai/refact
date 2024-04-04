import React from "react";
import { Section, Table, Container, Text } from "@radix-ui/themes";
import { Markdown } from "../Markdown";
import type { FIMContext } from "../../events";

export type ContextTableProps = {
  data: FIMContext;
};

export const ContextTable: React.FC<ContextTableProps> = ({ data }) => {
  return (
    <Section size="1">
      {data.was_looking_for && (
        <Table.Root variant="surface">
          <Table.Header>
            <Table.Row>
              <Table.ColumnHeaderCell>Symbol</Table.ColumnHeaderCell>
              <Table.ColumnHeaderCell>From</Table.ColumnHeaderCell>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {data.was_looking_for.map((item, index) => {
              return (
                <Table.Row key={index}>
                  <Table.RowHeaderCell>{item.symbol}</Table.RowHeaderCell>
                  <Table.Cell>{item.from}</Table.Cell>
                </Table.Row>
              );
            })}
          </Table.Body>
        </Table.Root>
      )}

      <Section size="1">
        {data.attached_files.map((file, i) => {
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
  );
};
