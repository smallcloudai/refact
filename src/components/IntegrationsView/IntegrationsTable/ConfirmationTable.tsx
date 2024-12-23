import React, { FC, useEffect, useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type { ColumnDef } from "@tanstack/react-table";
import { Button, Flex, Table, TextField } from "@radix-ui/themes";
import { PlusIcon } from "@radix-ui/react-icons";
import { toPascalCase } from "../../../utils/toPascalCase";

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

  const updateRow = (index: number, _field: string, value: string) => {
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
    field: string,
  ) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (isLastRow) {
        addRow();
      } else {
        const nextInput = document.querySelector<HTMLElement>(
          `[data-row-index="${rowIndex + 1}"][data-field="${field}"]`,
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
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const [value, setValue] = useState(initialValue);

      const onBlur = () => {
        updateRow(index, id, value);
      };

      // eslint-disable-next-line react-hooks/rules-of-hooks
      useEffect(() => {
        setValue(initialValue);
      }, [initialValue]);

      return (
        <TextField.Root
          value={value}
          size="1"
          data-row-index={index}
          data-field={id}
          onChange={(e) => setValue(e.target.value)}
          onBlur={onBlur}
          onKeyDown={(e) =>
            handleKeyPress(e, index === data.length - 1, index, id)
          }
        />
      );
    },
  };

  const columns: ColumnDef<string>[] = [
    {
      accessorKey: tableName,
      header: toPascalCase(tableName),
      size: 75,
    },
    {
      id: "actions",
      header: "",
      cell: ({ row }) => (
        <Flex gap="3" justify="start">
          <Button
            size="1"
            type="button"
            onClick={() => removeRow(row.index)}
            variant="outline"
            color="red"
          >
            Remove
          </Button>
          <Button
            onClick={addRow}
            type="button"
            size="1"
            variant="surface"
            color="gray"
          >
            <Flex align="stretch" gap="1">
              <PlusIcon /> Add row
            </Flex>
          </Button>
        </Flex>
      ),
    },
  ];

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
                    <Table.Cell key={cell.id}>
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
        {table.getRowModel().rows.length < 1 && (
          <Button
            onClick={addRow}
            type="button"
            size="1"
            variant="surface"
            color="gray"
            className={styles.addRowButton}
          >
            <Flex align="stretch" gap="1">
              <PlusIcon /> Add row
            </Flex>
          </Button>
        )}
      </Flex>
    </Flex>
  );
};
