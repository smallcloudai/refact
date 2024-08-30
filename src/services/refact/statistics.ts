import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

import { STATISTIC_URL } from "./consts";
import { RootState } from "../../app/store";

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
    getStatisticData: builder.query<StatisticData, undefined>({
      queryFn: async (_args, api, _opts, baseQuery) => {
        const getState = api.getState as () => RootState;
        const state = getState();
        const port = state.config.lspPort;
        const url = `http://127.0.0.1:${port}${STATISTIC_URL}`;
        const result = await baseQuery({
          url,
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) return { error: result.error };
        if (!isStatisticDataResponse(result.data)) {
          return {
            error: {
              data: result.data,
              error: "Invalid response from server",
              status: "CUSTOM_ERROR",
            },
          };
        }
        try {
          const json = JSON.parse(result.data.data) as StatisticData;
          return { data: json };
        } catch (e) {
          return {
            error: {
              data: result.data.data,
              error: "Invalid response from server",
              originalStatus: 200,
              status: "PARSING_ERROR",
            },
          };
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
