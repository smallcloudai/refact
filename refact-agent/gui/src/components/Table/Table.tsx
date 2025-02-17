import React from "react";
import { Box, Text } from "@radix-ui/themes";
import {
  ColumnName,
  RefactTableImpactLanguagesRow,
} from "../../services/refact";
import styles from "./Table.module.css";
import { Spinner } from "../Spinner";
import { TableRow } from "./TableRow";
import { TableCell } from "./TableCell";
import { formatTableCell } from "./formatTableCell";

const convertedColumnNames: Record<ColumnName, string> = {
  lang: "Lang.",
  refact: "Refact (char.)",
  human: "Human (char.)",
  total: "Total (char.)",
  refact_impact: "Refact Impact (%)",
  completions: "Compl.",
};

export const Table: React.FC<{
  refactImpactTable: RefactTableImpactLanguagesRow[] | null;
}> = ({ refactImpactTable }) => {
  if (refactImpactTable === null) {
    return <Spinner spinning />;
  }

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
                      {formatTableCell(
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
