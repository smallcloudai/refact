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

type EnvironmentVariablesTableProps = {
  initialData: Record<string, string>;
  onMCPEnvironmentVariables: (data: Record<string, string>) => void;
};

export const EnvironmentVariablesTable: FC<EnvironmentVariablesTableProps> = ({
  initialData,
  onMCPEnvironmentVariables,
}) => {
  const [data, setData] = useState<Record<string, string>>(initialData);

  const addRow = () => {
    setData((prev) => {
      const newKey = `key${Object.keys(prev).length}`;
      return { ...prev, [newKey]: "" };
    });
  };

  const removeRow = (index: number) => {
    setData((prev) => {
      const keys = Object.keys(prev);
      if (index >= 0 && index < keys.length) {
        const keyToRemove = keys[index];
        const { [keyToRemove]: _removed, ...rest } = prev;
        return rest;
      }
      return prev;
    });
  };

  const updateRow = (index: number, _field: keyof string, value: string) => {
    setData((prev) => {
      const keys = Object.keys(prev);
      if (index >= 0 && index < keys.length) {
        const key = keys[index];
        return { ...prev, [key]: value };
      }
      return prev;
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
    onMCPEnvironmentVariables(data);
  }, [data, onMCPEnvironmentVariables]);

  // const defaultColumn: Partial<ColumnDef<string>> = {
  //   cell: ({ row: { index }, column: { id } }) => {
  //     const initialValue = data[index];

  //     return (
  //       <DefaultCell
  //         initialValue={initialValue}
  //         data={data}
  //         index={index}
  //         id={id}
  //         updateRow={updateRow}
  //         handleKeyPress={handleKeyPress}
  //       />
  //     );
  //   },
  // };

  // const columns = useMemo<ColumnDef<string>[]>(
  //   () => [
  //     {
  //       accessorKey: "args",
  //       header: "MCP Arguments",
  //     },
  //     {
  //       id: "actions",
  //       header: "",
  //       cell: ({ row }) => (
  //         <Flex gap="3" width="100%">
  //           <Button
  //             size="1"
  //             type="button"
  //             onClick={() => removeRow(row.index)}
  //             variant="outline"
  //             color="red"
  //           >
  //             Remove
  //           </Button>
  //         </Flex>
  //       ),
  //     },
  //   ],
  //   // need to keep track of length of data array to be sure that it is always up to date
  //   // eslint-disable-next-line react-hooks/exhaustive-deps
  //   [data.length],
  // );

  // // const table = useReactTable({
  // //   data,
  // //   columns,
  // //   defaultColumn,
  // //   getCoreRowModel: getCoreRowModel(),
  // // });

  type EnvVarRow = {
    key: string;
    value: string;
  };

  // Add this before the table setup
  const tableData = useMemo(
    () =>
      Object.entries(data).map(
        ([key, value]): EnvVarRow => ({
          key,
          value,
        }),
      ),
    [data],
  );

  const columns = useMemo<ColumnDef<EnvVarRow>[]>(
    () => [
      {
        id: "key",
        header: "Environment Variable",
        cell: ({ row: { id, index } }) => (
          <DefaultCell
            initialValue={id}
            data={data}
            index={index}
            id="key"
            updateRow={updateRow}
            handleKeyPress={handleKeyPress}
          />
        ),
      },
      {
        id: "value",
        header: "Value",
        cell: ({ row: { id, index } }) => (
          <DefaultCell
            initialValue={data[id]}
            data={data}
            index={index}
            id="value"
            updateRow={updateRow}
            handleKeyPress={handleKeyPress}
          />
        ),
      },
      {
        id: "actions",
        header: "",
        cell: ({ row: { index } }) => (
          <Flex gap="3" width="100%">
            <Button
              size="1"
              type="button"
              onClick={() => removeRow(index)}
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
