import React from "react";
import { useAppSelector } from "../../hooks";
import { selectConfig } from "../Config/configSlice";
import { selectConnections } from "./connectionStatusSlice";
import { HoverCard, Table, Text, Flex } from "@radix-ui/themes";

export const ConnectionStatus: React.FC = () => {
  const config = useAppSelector(selectConfig);
  const connections = useAppSelector(selectConnections, {
    devModeChecks: { stabilityCheck: "never" },
  });
  if (config.host !== "web" && config.features?.connections !== true) return;

  return (
    <Flex justify="end">
      <HoverCard.Root open={connections.length === 0 ? false : undefined}>
        <HoverCard.Trigger>
          <Text size="1">sockets: {connections.length}</Text>
        </HoverCard.Trigger>
        <HoverCard.Content>
          <Table.Root>
            <Table.Header>
              <Table.Row>
                <Table.ColumnHeaderCell>Name</Table.ColumnHeaderCell>
                <Table.ColumnHeaderCell>id</Table.ColumnHeaderCell>
                <Table.ColumnHeaderCell>Status</Table.ColumnHeaderCell>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {connections.map((connection) => {
                return (
                  <Table.Row key={connection.id}>
                    <Table.RowHeaderCell>{connection.name}</Table.RowHeaderCell>
                    <Table.Cell>{connection.id}</Table.Cell>
                    <Table.Cell>{connection.status}</Table.Cell>
                  </Table.Row>
                );
              })}
            </Table.Body>
          </Table.Root>
        </HoverCard.Content>
      </HoverCard.Root>
    </Flex>
  );
};
