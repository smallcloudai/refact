import { useState, useEffect, useMemo } from "react";
import type { FocusEvent } from "react";
import { TextField } from "@radix-ui/themes";

type DefaultCellProps<TData> = {
  initialValue: string;
  updateRow: (index: number, field: keyof TData, value: string) => void;
  index: number;
  id: string;
  data: TData[];
  handleKeyPress: (
    e: React.KeyboardEvent<HTMLInputElement>,
    isLastRow: boolean,
    rowIndex: number,
    field: keyof TData,
    value: string,
  ) => void;
};

export const DefaultCell = <TData,>({
  initialValue,
  updateRow,
  index,
  id,
  data,
  handleKeyPress,
}: DefaultCellProps<TData>) => {
  const [value, setValue] = useState(initialValue);

  const onBlur = (_event: FocusEvent<HTMLInputElement>) => {
    updateRow(index, id as keyof TData, value);
  };

  useEffect(() => {
    setValue(initialValue);
  }, [initialValue]);

  const isLastRow = useMemo(
    () => index === data.length - 1,
    [index, data.length],
  );

  return (
    <TextField.Root
      value={value}
      size="1"
      data-row-index={index}
      data-field={id}
      onChange={(e) => setValue(e.target.value)}
      onBlur={(e) => onBlur(e)}
      onKeyDown={(e) =>
        handleKeyPress(e, isLastRow, index, id as keyof TData, value)
      }
    />
  );
};
