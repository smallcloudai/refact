import { FormatCellValue } from "../../services/refact";

function convertNumber(value: number): number {
  return Number.isInteger(value) ? value : +value.toFixed(2);
}
export const formatCellNumber = (cellValue: number | string): string => {
  cellValue = Number(cellValue);

  if (cellValue >= 1e6) {
    const roundedNumber = cellValue / 1e6;
    return `${convertNumber(roundedNumber)}M`;
  } else if (cellValue < 1e3) {
    return `${cellValue}`;
  } else {
    const roundedNumber = cellValue / 1000; // 1.06002
    return `${convertNumber(roundedNumber)}k`;
  }
};

export const formatTableCell: FormatCellValue = (
  columnName: string,
  cellValue: string | number,
): string | number => {
  if (columnName === "refact_impact") {
    return cellValue === 0
      ? cellValue
      : parseFloat(cellValue.toString()).toFixed(2);
  } else if (columnName === "lang" || columnName === "completions") {
    return cellValue;
  } else {
    return formatCellNumber(cellValue);
  }
};
