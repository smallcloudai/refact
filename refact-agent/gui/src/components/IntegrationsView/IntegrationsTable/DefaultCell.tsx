import { useState, useEffect } from "react";
import type { FocusEvent, KeyboardEvent } from "react";
import { TextField } from "@radix-ui/themes";

type DefaultCellProps = {
  initialValue: string;
  onChange: (value: string) => void;
  onKeyPress: (e: KeyboardEvent<HTMLInputElement>) => void;
  "data-row-index"?: number;
  "data-field"?: string;
  "data-next-row"?: string;
};

export const DefaultCell = ({
  initialValue,
  onChange,
  onKeyPress,
  "data-row-index": dataRowIndex,
  "data-field": dataField,
  "data-next-row": dataNextRow,
}: DefaultCellProps) => {
  const [value, setValue] = useState(initialValue);

  const onBlur = (_event: FocusEvent<HTMLInputElement>) => {
    onChange(value);
  };

  useEffect(() => {
    setValue(initialValue);
  }, [initialValue]);

  return (
    <TextField.Root
      value={value}
      size="1"
      data-row-index={dataRowIndex}
      data-field={dataField}
      data-next-row={dataNextRow}
      onChange={(e) => setValue(e.target.value)}
      onBlur={onBlur}
      onKeyDown={onKeyPress}
    />
  );
};
