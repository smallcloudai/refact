import React, { FC, useEffect, useState, useMemo } from "react";
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type { ColumnDef } from "@tanstack/react-table";
import { Button, Flex, Table } from "@radix-ui/themes";
import { PlusIcon } from "@radix-ui/react-icons";
import { toPascalCase } from "../../../utils/toPascalCase";
import { DefaultCell } from "./DefaultCell";

import styles from "./ConfirmationTable.module.css";

type ConfirmationTableProps = {
  tableName: string;
  initialData: string[];
  onToolConfirmation: (key: string, data: string[]) => void;
};

export const ConfirmationTable: FC<ConfirmationTableProps> = ({
  tableName,
  initialData,
  onToolConfirmation,
}) => {
  const [data, setData] = useState<string[]>(initialData);

  const addRow = () => {
    setData((prev) => {
      return [...prev, ""];
    });
  };

  const removeRow = (index: number) => {
    setData((prev) => prev.filter((_, i) => i !== index));
  };

  const updateRow = (index: number, _field: keyof string, value: string) => {
    setData((prev) => {
      return prev.map((row, i) => {
        if (i === index) {
          return value;
        }
        return row;
      });
    });
  };

  const handleKeyPress = (
    e: React.KeyboardEvent<HTMLInputElement>,
    isLastRow: boolean,
    rowIndex: number,
    field: keyof string,
    value: string,
  ) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (isLastRow) {
        updateRow(rowIndex, field, value);
        addRow();
      } else {
        // TODO: since we cannot not listen for data.length change, reference is dropped and we cannot focus on the next input
        const nextInput = document.querySelector<HTMLElement>(
          `[data-row-index="${rowIndex + 1}"][data-field="${field as string}"]`,
        );
        nextInput?.focus();
      }
    }
  };

  useEffect(() => {
    onToolConfirmation(tableName, data);
  }, [tableName, data, onToolConfirmation]);

  const defaultColumn: Partial<ColumnDef<string>> = {
    cell: ({ row: { index }, column: { id } }) => {
      const initialValue = data[index];

      return (
        <DefaultCell
          initialValue={initialValue}
          data={data}
          index={index}
          id={id}
          updateRow={updateRow}
          handleKeyPress={handleKeyPress}
        />
      );
    },
  };

  const columns = useMemo<ColumnDef<string>[]>(
    () => [
      {
        accessorKey: tableName,
        header: toPascalCase(tableName),
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
    // need to keep track of length of data array to be sure that it is always up to date
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [tableName, data.length],
  );

  const table = useReactTable({
    data,
    columns,
    defaultColumn,
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
                  No rules set yet
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
