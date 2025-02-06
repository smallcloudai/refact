import React, { FC, useEffect, useState, useMemo } from "react";
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type { ColumnDef } from "@tanstack/react-table";
import { Button, Flex, Table } from "@radix-ui/themes";
import { PlusIcon } from "@radix-ui/react-icons";
import { DefaultCell } from "./DefaultCell";

import styles from "./ConfirmationTable.module.css";
import { debugIntegrations } from "../../../debugConfig";
import { MCPEnvs } from "../../../events";

type EnvironmentVariablesTableProps = {
  initialData: MCPEnvs;
  onMCPEnvironmentVariables: (data: MCPEnvs) => void;
};

type EnvVarRow = {
  key: string;
  value: string;
  originalKey: string; // Keep track of original key for updates
};

export const EnvironmentVariablesTable: FC<EnvironmentVariablesTableProps> = ({
  initialData,
  onMCPEnvironmentVariables,
}) => {
  const [data, setData] = useState<MCPEnvs>(initialData);

  const addRow = () => {
    setData((prev) => {
      const newKey = `${Object.keys(prev).length}`;
      return { ...prev, [newKey]: "" };
    });
  };

  const removeRow = (originalKey: string) => {
    setData((prev) => {
      const { [originalKey]: _removed, ...rest } = prev;
      return rest;
    });
  };

  const updateRow = (
    originalKey: string,
    field: "key" | "value",
    newValue: string,
  ) => {
    setData((prev) => {
      if (field === "key") {
        // When updating key, we need to create a new entry and remove the old one
        const { [originalKey]: value, ...rest } = prev;
        return { ...rest, [newValue]: value };
      } else {
        // When updating value, just update the value for the existing key
        return { ...prev, [originalKey]: newValue };
      }
    });
  };

  const handleKeyPress = (
    e: React.KeyboardEvent<HTMLInputElement>,
    isLastRow: boolean,
    originalKey: string,
    field: "key" | "value",
    value: string,
  ) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (isLastRow) {
        updateRow(originalKey, field, value);
        addRow();
      } else {
        const nextInput = document.querySelector<HTMLElement>(
          `[data-next-row="${originalKey}"][data-field="${field}"]`,
        );
        nextInput?.focus();
      }
    }
  };

  useEffect(() => {
    onMCPEnvironmentVariables(data);
  }, [data, onMCPEnvironmentVariables]);

  const tableData = useMemo(
    () =>
      Object.entries(data).map(
        ([key, value]): EnvVarRow => ({
          key,
          value,
          originalKey: key,
        }),
      ),
    [data],
  );

  useEffect(() => {
    debugIntegrations(`[DEBUG MCP]: envs table data: `, tableData);
  }, [tableData]);

  const columns = useMemo<ColumnDef<EnvVarRow>[]>(
    () => [
      {
        id: "key",
        header: "Environment Variable",
        cell: ({ row }) => (
          <DefaultCell
            initialValue={row.original.key}
            data-row-index={row.index}
            data-field="key"
            data-next-row={row.original.originalKey}
            onChange={(value) =>
              updateRow(row.original.originalKey, "key", value)
            }
            onKeyPress={(e) =>
              handleKeyPress(
                e,
                row.index === tableData.length - 1,
                row.original.originalKey,
                "key",
                e.currentTarget.value,
              )
            }
          />
        ),
      },
      {
        id: "value",
        header: "Value",
        cell: ({ row }) => (
          <DefaultCell
            initialValue={row.original.value}
            data-row-index={row.index}
            data-field="value"
            data-next-row={row.original.originalKey}
            onChange={(value) =>
              updateRow(row.original.originalKey, "value", value)
            }
            onKeyPress={(e) =>
              handleKeyPress(
                e,
                row.index === tableData.length - 1,
                row.original.originalKey,
                "value",
                e.currentTarget.value,
              )
            }
          />
        ),
      },
      {
        id: "actions",
        header: "",
        cell: ({ row }) => (
          <Flex gap="3" width="100%">
            <Button
              size="1"
              type="button"
              onClick={() => removeRow(row.original.originalKey)}
              variant="outline"
              color="red"
            >
              Remove
            </Button>
          </Flex>
        ),
      },
    ],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [tableData.length],
  );

  const table = useReactTable({
    data: tableData,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <Flex direction="column" gap="2" mb="1" width="100%">
      <Flex direction="column" gap="2" mb="1" width="100%">
        <Table.Root size="1">
          <Table.Header>
            {table.getHeaderGroups().map((headerGroup) => (
              <Table.Row key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <Table.ColumnHeaderCell key={header.id}>
                    {flexRender(
                      header.column.columnDef.header,
                      header.getContext(),
                    )}
                  </Table.ColumnHeaderCell>
                ))}
              </Table.Row>
            ))}
          </Table.Header>
          <Table.Body>
            {table.getRowModel().rows.length ? (
              table.getRowModel().rows.map((row) => (
                <Table.Row key={row.id}>
                  {row.getVisibleCells().map((cell) => (
                    <Table.Cell
                      key={cell.id}
                      className={
                        cell.column.id === "actions"
                          ? styles.actionCell
                          : undefined
                      }
                    >
                      {flexRender(
                        cell.column.columnDef.cell,
                        cell.getContext(),
                      )}
                    </Table.Cell>
                  ))}
                </Table.Row>
              ))
            ) : (
              <Table.Row>
                <Table.Cell colSpan={columns.length}>
                  No environment variables set yet
                </Table.Cell>
              </Table.Row>
            )}
          </Table.Body>
        </Table.Root>
        <Button
          onClick={addRow}
          type="button"
          size="1"
          variant="surface"
          color="gray"
          className={styles.addRowButtonAlignedOnStart}
        >
          <Flex align="stretch" gap="1">
            <PlusIcon /> Add row
          </Flex>
        </Button>
      </Flex>
    </Flex>
  );
};
