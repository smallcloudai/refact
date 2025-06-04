import { FC, useEffect, useState, useMemo, useRef, useCallback } from "react";
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
import isEqual from "lodash.isequal";

type ParametersTableProps = {
  initialData: ToolParameterEntity[];
  onToolParameters: (data: ToolParameterEntity[]) => void;
};

type ParameterRow = ToolParameterEntity & {
  index: number;
};

export const ParametersTable: FC<ParametersTableProps> = ({
  initialData,
  onToolParameters,
}) => {
  const [data, setData] = useState<ToolParameterEntity[]>(initialData);
  const [validateError, setValidateError] = useState<string | null>(null);
  const previousDataRef = useRef<ToolParameterEntity[]>(initialData);
  const previousInitialDataRef = useRef<ToolParameterEntity[]>(initialData);

  // Sync with initialData when it changes from parent
  useEffect(() => {
    if (!isEqual(previousInitialDataRef.current, initialData)) {
      previousInitialDataRef.current = initialData;
      setData(initialData);
    }
  }, [initialData]);

  const addRow = useCallback(() => {
    setData((prev) => [...prev, { name: "", description: "", type: "string" }]);
  }, []);

  const removeRow = useCallback((index: number) => {
    setData((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const validateAndUpdateField = useCallback(
    (field: keyof ToolParameterEntity, value: string) => {
      if (field === "name") {
        if (!validateSnakeCase(value)) {
          debugTables(
            `[DEBUG VALIDATION]: field ${field} is not written in snake case`,
          );
          setValidateError(
            `The value "${value}" must be written in snake case.`,
          );
        } else {
          setValidateError(null);
        }
      }
      return value;
    },
    [],
  );

  const updateRow = useCallback(
    (index: number, field: keyof ToolParameterEntity, value: string) => {
      debugTables(`[DEBUG]: updating data of the table`);
      const validatedValue = validateAndUpdateField(field, value);

      setData((prev) =>
        prev.map((row, i) =>
          i === index ? { ...row, [field]: validatedValue } : row,
        ),
      );
    },
    [validateAndUpdateField],
  );

  useEffect(() => {
    // Only call onToolParameters if data has actually changed
    if (!isEqual(previousDataRef.current, data)) {
      previousDataRef.current = data;
      onToolParameters(data);
    }
  }, [data, onToolParameters]);

  const tableData = useMemo<ParameterRow[]>(
    () => data.map((row, index) => ({ ...row, index })),
    [data],
  );

  const columns = useMemo<ColumnDef<ParameterRow>[]>(
    () => [
      {
        id: "name",
        header: "Name",
        cell: ({ row }) => {
          const isLastRow = row.index === data.length - 1;

          return (
            <DefaultCell
              initialValue={row.original.name}
              data-row-index={row.index}
              data-field="name"
              data-next-row={row.index.toString()}
              onChange={(value) => updateRow(row.index, "name", value)}
              onKeyPress={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  if (isLastRow) {
                    updateRow(row.index, "name", e.currentTarget.value);
                    addRow();
                  } else {
                    const nextInput = document.querySelector<HTMLElement>(
                      `[data-row-index="${row.index + 1}"][data-field="name"]`,
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
        id: "description",
        header: "Description",
        cell: ({ row }) => {
          const isLastRow = row.index === data.length - 1;

          return (
            <DefaultCell
              initialValue={row.original.description}
              data-row-index={row.index}
              data-field="description"
              data-next-row={row.index.toString()}
              onChange={(value) => updateRow(row.index, "description", value)}
              onKeyPress={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  if (isLastRow) {
                    updateRow(row.index, "description", e.currentTarget.value);
                    addRow();
                  } else {
                    const nextInput = document.querySelector<HTMLElement>(
                      `[data-row-index="${
                        row.index + 1
                      }"][data-field="description"]`,
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
    [data.length, updateRow, addRow, removeRow],
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
