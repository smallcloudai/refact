import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

import { STATISTIC_URL } from "./consts";
import { RootState } from "../../app/store";

// TODO: this could be for the whole lsp?
// Add port
export const statisticsApi = createApi({
  reducerPath: "statisticsApi",

  baseQuery: fetchBaseQuery({
    prepareHeaders: (headers, api) => {
      const getState = api.getState as () => RootState;
      const state = getState();
      const token = state.config.apiKey;
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  endpoints: (builder) => ({
    getStatisticData: builder.query<StatisticData, { port: number }>({
      query: ({ port }) => `http://127.0.0.1:${port}${STATISTIC_URL}`,
      transformResponse: (response: unknown): StatisticData => {
        if (!isStatisticDataResponse(response)) {
          throw new Error("Invalid response for statistic data");
        }
        try {
          return JSON.parse(response.data) as StatisticData;
        } catch {
          throw new Error("Invalid response for statistic data");
        }
      },
    }),
  }),
  refetchOnMountOrArgChange: true,
});

export type RefactTableImpactDateObj = {
  completions: number;
  human: number;
  langs: string[];
  refact: number;
  refact_impact: number;
  total: number;
};
export type RefactTableImpactLanguagesRow = {
  [key in ColumnName]: string | number;
};
export type StatisticData = {
  refact_impact_dates: {
    data: {
      daily: Record<string, RefactTableImpactDateObj>;
      weekly: Record<string, RefactTableImpactDateObj>;
    };
  };
  table_refact_impact: {
    columns: string[];
    data: RefactTableImpactLanguagesRow[];
    title: string;
  };
};

export type ColumnName =
  | "lang"
  | "refact"
  | "human"
  | "total"
  | "refact_impact"
  | "completions";

export type CellValue = string | number;

export type FormatCellValue = (
  columnName: string,
  cellValue: string | number,
) => string | number;

export function isStatisticDataResponse(
  json: unknown,
): json is { data: string } {
  if (!json || typeof json !== "object") return false;
  if (!("data" in json)) return false;
  return typeof json.data === "string";
}
