import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

import { STATISTIC_URL } from "./consts";

export const statisticsApi = createApi({
  reducerPath: "statisticsApi",

  baseQuery: fetchBaseQuery({
    // TODO: set this to the configured lsp url
    baseUrl: "",
  }),
  endpoints: (builder) => ({
    getStatisticData: builder.query<StatisticData, undefined>({
      query: () => STATISTIC_URL,
      transformResponse: (response: unknown): StatisticData => {
        if (!isStatisticDataResponse(response)) {
          throw new Error("Invalid response for statistic data");
        }
        return JSON.parse(response.data) as StatisticData;
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

export async function getStatisticData(
  lspUrl?: string,
): Promise<{ data: string }> {
  const statisticDataEndpoint = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${STATISTIC_URL}`
    : STATISTIC_URL;
  const response = await fetch(statisticDataEndpoint, {
    method: "GET",
    credentials: "same-origin",
    headers: {
      accept: "application/json",
    },
  });
  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const json: unknown = await response.json();
  if (!isStatisticDataResponse(json)) {
    throw new Error("Invalid response for statistic data");
  }
  return json;
}
