// TODO: a lot of duplicative code is here between ParametersTable and ConfirmationTable components

import React, { FC, useEffect, useState, useMemo } from "react";
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type { ColumnDef } from "@tanstack/react-table";
import { Button, Flex, Table, Text } from "@radix-ui/themes";
import { ToolParameterEntity } from "../../../services/refact";
import { PlusIcon } from "@radix-ui/react-icons";
import { validateSnakeCase } from "../../../utils/validateSnakeCase";
import { debugTables } from "../../../debugConfig";
import { DefaultCell } from "./DefaultCell";

type ParametersTableProps = {
  initialData: ToolParameterEntity[];
  onToolParameters: (data: ToolParameterEntity[]) => void;
};

export const ParametersTable: FC<ParametersTableProps> = ({
  initialData,
  onToolParameters,
}) => {
  const [data, setData] = useState<ToolParameterEntity[]>(initialData);
  const [validateError, setValidateError] = useState<string | null>(null);

  const addRow = () => {
    setData((prev) => [...prev, { name: "", description: "", type: "string" }]);
  };

  const removeRow = (index: number) => {
    setData((prev) => prev.filter((_, i) => i !== index));
  };

  const updateRow = (
    index: number,
    field: keyof ToolParameterEntity,
    value: string,
  ) => {
    debugTables(`[DEBUG]: updating data of the table`);
    if (field === "name" && !validateSnakeCase(value)) {
      debugTables(
        `[DEBUG VALIDATION]: field ${field} is not written in snake case`,
      );
      setValidateError(`The field "${value}" must be written in snake case.`);
    } else if (field === "name" && validateSnakeCase(value)) {
      setValidateError(null);
    }

    setData((prev) =>
      prev.map((row, i) => (i === index ? { ...row, [field]: value } : row)),
    );
  };

  const handleKeyPress = (
    e: React.KeyboardEvent<HTMLInputElement>,
    isLastRow: boolean,
    rowIndex: number,
    field: keyof ToolParameterEntity,
    value: string,
  ) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (isLastRow) {
        updateRow(rowIndex, field, value);
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
    onToolParameters(data);
  }, [data, onToolParameters]);

  const defaultColumn: Partial<ColumnDef<ToolParameterEntity>> = {
    cell: ({ getValue, row: { index }, column: { id } }) => {
      const initialValue = getValue() as string;

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

  const columns = useMemo<ColumnDef<ToolParameterEntity>[]>(
    () => [
      {
        accessorKey: "name",
        header: "Name",
      },
      {
        accessorKey: "description",
        header: "Description",
      },
      {
        id: "actions",
        header: "",
        cell: ({ row }) => (
          <Button
            size="1"
            type="button"
            onClick={() => removeRow(row.index)}
            variant="outline"
            color="red"
          >
            Remove
          </Button>
        ),
      },
    ],
    // need to keep track of length of data array to be sure that it is always up to date
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [data.length],
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
                  No parameters set yet
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
        >
          <Flex align="stretch" gap="1">
            <PlusIcon /> Add row
          </Flex>
        </Button>
      </Flex>
      {validateError && (
        <Text color="red" size="2">
          {validateError}
        </Text>
      )}
    </Flex>
  );
};
