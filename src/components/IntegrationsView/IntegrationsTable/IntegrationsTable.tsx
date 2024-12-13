import React, { useState } from "react";
import { useTable, flexRender, getCoreRowModel } from "@tanstack/react-table";

// Определение типа данных, получаемых с сервера
export type ToolParameterEntity = {
  name: string;
  description: string;
  type?: string; // Это поле отображаться в таблице не будет
};

// Пропсы для компонента
interface EditableTableProps {
  initialData: ToolParameterEntity[];
}

const IntegrationsTable: React.FC<EditableTableProps> = ({ initialData }) => {
  // Состояние таблицы
  const [data, setData] = useState<ToolParameterEntity[]>(initialData);

  // Добавить строку
  const addRow = () => {
    setData((prev) => [
      ...prev,
      { name: "", description: "" }, // type не сохраняется, ибо без него сервер вернёт нам тип самостоятельно исходя из name и description
    ]);
  };

  // Удалить строку
  const removeRow = (index: number) => {
    setData((prev) => prev.filter((_, i) => i !== index));
  };

  // Обновить данные в строке
  const updateRow = (
    index: number,
    field: keyof ToolParameterEntity,
    value: string,
  ) => {
    setData((prev) =>
      prev.map((row, i) => (i === index ? { ...row, [field]: value } : row)),
    );
  };

  // Обработка клавиши Enter
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

  // Определение колонок таблицы
  const columns = [
    {
      accessorKey: "name",
      header: "Name",
      cell: ({ row, getValue }: any) => {
        const isLastRow = row.index === data.length - 1;
        return (
          <RadixInput.Root>
            <RadixInput.Input
              data-row-index={row.index}
              data-field="name"
              value={getValue()}
              onChange={(e) => updateRow(row.index, "name", e.target.value)}
              onKeyDown={(e) => handleKeyPress(e, isLastRow, row.index, "name")}
            />
          </RadixInput.Root>
        );
      },
    },
    {
      accessorKey: "description",
      header: "Description",
      cell: ({ row, getValue }: any) => {
        const isLastRow = row.index === data.length - 1;
        return (
          <RadixInput.Root>
            <RadixInput.Input
              data-row-index={row.index}
              data-field="description"
              value={getValue()}
              onChange={(e) =>
                updateRow(row.index, "description", e.target.value)
              }
              onKeyDown={(e) =>
                handleKeyPress(e, isLastRow, row.index, "description")
              }
            />
          </RadixInput.Root>
        );
      },
    },
    {
      id: "actions",
      header: "Remove",
      cell: ({ row }: any) => (
        <RadixButton.Root onClick={() => removeRow(row.index)}>
          Remove
        </RadixButton.Root>
      ),
    },
  ];

  const table = useTable({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  return (
    <div>
      <RadixTable.Root>
        <RadixTable.Header>
          {table.getHeaderGroups().map((headerGroup) => (
            <RadixTable.Row key={headerGroup.id}>
              {headerGroup.headers.map((header) => (
                <RadixTable.ColumnHeader key={header.id}>
                  {flexRender(
                    header.column.columnDef.header,
                    header.getContext(),
                  )}
                </RadixTable.ColumnHeader>
              ))}
            </RadixTable.Row>
          ))}
        </RadixTable.Header>
        <RadixTable.Body>
          {table.getRowModel().rows.map((row) => (
            <RadixTable.Row key={row.id}>
              {row.getVisibleCells().map((cell) => (
                <RadixTable.Cell key={cell.id}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </RadixTable.Cell>
              ))}
            </RadixTable.Row>
          ))}
        </RadixTable.Body>
      </RadixTable.Root>
      <RadixButton.Root onClick={addRow}>Add Row</RadixButton.Root>
    </div>
  );
};

export default IntegrationsTable;
