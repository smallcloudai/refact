import React, { useEffect, useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
} from "@tanstack/react-table";
import type { ColumnDef } from "@tanstack/react-table";
import { Button, Flex, Table, Text, TextField } from "@radix-ui/themes";
import { ToolParameterEntity } from "../../../services/refact";
import { PlusIcon } from "@radix-ui/react-icons";
import { validateSnakeCase } from "../../../utils/validateSnakeCase";
import { debugIntegrations } from "../../../debugConfig";

type IntegrationsTableProps = {
  initialData: ToolParameterEntity[];
  onToolParameters: (data: ToolParameterEntity[]) => void;
};

const IntegrationsTable: React.FC<IntegrationsTableProps> = ({
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
    debugIntegrations(`[DEBUG UPDATE ROW]: updating row, field: ${field}`);
    setValidateError(null);
    if (field === "name" && !validateSnakeCase(value)) {
      debugIntegrations(
        `[DEBUG VALIDATION]: field ${field} is not written in snake case`,
      );
      setValidateError(`The field "${value}" must be written in snake case.`);
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
    onToolParameters(data);
  }, [data, onToolParameters]);

  const defaultColumn: Partial<ColumnDef<ToolParameterEntity>> = {
    cell: ({ getValue, row: { index }, column: { id } }) => {
      const initialValue = getValue() as string;
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const [value, setValue] = useState(initialValue);

      const onBlur = () => {
        updateRow(index, id as keyof ToolParameterEntity, value);
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
            handleKeyPress(
              e,
              index === data.length - 1,
              index,
              id as keyof ToolParameterEntity,
            )
          }
        />
      );
    },
  };

  const columns: ColumnDef<ToolParameterEntity>[] = [
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

export default IntegrationsTable;
