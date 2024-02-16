import React from "react";
import { Box, Text } from "@radix-ui/themes";
import {
  FormatCellValue,
  ColumnName,
  RefactTableImpactLanguagesRow,
  RefactTableData,
} from "../../services/refact";
import styles from "./Table.module.css";
import { Spinner } from "../Spinner";
import { TableRow } from "./TableRow";
import { TableCell } from "./TableCell";

export const Table: React.FC<{
  refactTable: RefactTableData | null;
}> = ({ refactTable }) => {
  if (refactTable === null) {
    return <Spinner />;
  }
  const refactImpactTable: RefactTableImpactLanguagesRow[] =
    refactTable.table_refact_impact.data;
  const convertedColumnNames: Record<ColumnName, string> = {
    lang: "Lang.",
    refact: "Refact",
    human: "Human",
    total: "Total",
    refact_impact: "Refact Impact",
    completions: "Compl.",
  };
  const formatCellValue: FormatCellValue = (
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
      const convertedNumber = Number(
        cellValue
          .toLocaleString("en-US", {
            style: "decimal",
            maximumFractionDigits: 0,
            minimumFractionDigits: 0,
            useGrouping: true,
          })
          .replace(",", "."),
      );
      if (convertedNumber === 0) {
        return "0";
      } else if (Number.isInteger(convertedNumber)) {
        return convertedNumber + "k";
      } else {
        return `${convertedNumber.toFixed(2)}k`;
      }
    }
  };

  return (
    <Box>
      <Text as="p" size="2" mb="1">
        Refact&apos;s impact by language
      </Text>
      <table className={styles.table}>
        <thead>
          <TableRow>
            {Object.values(convertedColumnNames).map(
              (columnName: string, idx: number) => (
                <TableCell key={idx} className={styles.tableCellHead}>
                  {columnName}
                </TableCell>
              ),
            )}
          </TableRow>
        </thead>
        <tbody>
          {refactImpactTable.map(
            (rowData: RefactTableImpactLanguagesRow, idx: number) => (
              <TableRow key={idx}>
                {Object.keys(convertedColumnNames).map(
                  (columnName: string, idx: number) => (
                    <TableCell key={idx}>
                      {formatCellValue(
                        columnName,
                        rowData[
                          columnName as keyof RefactTableImpactLanguagesRow
                        ],
                      )}
                    </TableCell>
                  ),
                )}
              </TableRow>
            ),
          )}
        </tbody>
      </table>
    </Box>
  );
};
