import React from "react";
import { useAppSelector } from "../../hooks";
import { selectConfig } from "../Config/configSlice";
import { selectConnections } from "./connectionStatusSlice";
import { HoverCard, Table, Box, Text } from "@radix-ui/themes";

export const ConnectionStatus: React.FC = () => {
  const config = useAppSelector(selectConfig);
  const connections = useAppSelector(selectConnections);

  if (config.host !== "web" && config.features?.connections !== true) return;

  return (
    <Box>
      <HoverCard.Root>
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
    </Box>
  );
};
