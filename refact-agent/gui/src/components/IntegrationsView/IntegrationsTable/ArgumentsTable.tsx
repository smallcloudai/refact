import { FC, useEffect, useState, useMemo } from "react";
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

type ArgumentsTableProps = {
  initialData: string[];
  onMCPArguments: (data: string[]) => void;
};

type ArgumentRow = {
  value: string;
  index: number;
};

export const ArgumentsTable: FC<ArgumentsTableProps> = ({
  initialData,
  onMCPArguments,
}) => {
  const [data, setData] = useState<string[]>(initialData);

  const addRow = () => {
    setData((prev) => [...prev, ""]);
  };

  const removeRow = (index: number) => {
    setData((prev) => prev.filter((_, i) => i !== index));
  };

  const updateRow = (index: number, value: string) => {
    setData((prev) => prev.map((row, i) => (i === index ? value : row)));
  };

  useEffect(() => {
    onMCPArguments(data);
  }, [data, onMCPArguments]);

  const tableData = useMemo<ArgumentRow[]>(
    () => data.map((value, index) => ({ value, index })),
    [data],
  );

  const columns = useMemo<ColumnDef<ArgumentRow>[]>(
    () => [
      {
        id: "argument",
        header: "MCP Arguments",
        cell: ({ row }) => {
          const isLastRow = row.index === data.length - 1;

          return (
            <DefaultCell
              initialValue={row.original.value}
              data-row-index={row.index}
              data-field="argument"
              data-next-row={row.index.toString()}
              onChange={(value) => updateRow(row.index, value)}
              onKeyPress={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  if (isLastRow) {
                    updateRow(row.index, e.currentTarget.value);
                    addRow();
                  } else {
                    const nextInput = document.querySelector<HTMLElement>(
                      `[data-row-index="${
                        row.index + 1
                      }"][data-field="argument"]`,
                    );
                    nextInput?.focus();
                  }
                }
              }}
            />
          );
        },
      },
      {
        id: "actions",
        header: "",
        cell: ({ row }) => (
          <Flex gap="3" width="100%">
            <Button
              size="1"
              type="button"
              onClick={() => removeRow(row.index)}
              variant="outline"
              color="red"
            >
              Remove
            </Button>
          </Flex>
        ),
      },
    ],
    [data.length],
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
                  No arguments set yet
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
