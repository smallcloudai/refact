import { DiffChunkWithTypeAndApply } from "../../components/ChatContent/DiffContent";
import { getApiKey, parseOrElse } from "../../utils";
import { DIFF_APPLY_URL, DIFF_UNDO_URL, DEFF_APPLIED_CHUNKS } from "./consts";
import { DiffChunk } from "./types";

export interface DiffAppliedStateResponse {
  id: number;
  state: number[];
}

export async function checkDiff(
  chunks: DiffChunk[],
  lspUrl?: string,
): Promise<DiffAppliedStateResponse> {
  const addr = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${DEFF_APPLIED_CHUNKS}`
    : DEFF_APPLIED_CHUNKS;

  const apiKey = getApiKey();

  const response = await fetch(addr, {
    method: "POST",
    body: JSON.stringify({ chunks }),
    credentials: "same-origin",
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    headers: {
      accept: "application/json",
      ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
    },
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const text = await response.text();

  const json = parseOrElse<DiffAppliedStateResponse>(text, {
    id: 0,
    state: [],
  });

  return json;
}

interface DiffOperationResponse {
  fuzzy_results: {
    chunk_id: number;
    fuzzy_n_used: number;
  }[];

  state: number[];
}

export async function doDiff(
  opperation: "add" | "remove",
  chunks: DiffChunkWithTypeAndApply[],
  lspUrl?: string,
): Promise<DiffOperationResponse> {
  const url = opperation === "remove" ? DIFF_UNDO_URL : DIFF_APPLY_URL;
  const addr = lspUrl ? `${lspUrl.replace(/\/*$/, "")}${url}` : url;

  const apiKey = getApiKey();

  const apply = chunks.map((d) => d.apply);

  const response = await fetch(addr, {
    method: "POST",
    body: JSON.stringify({
      apply,
      chunks,
    }),
    credentials: "same-origin",
    redirect: "follow",
    cache: "no-cache",
    referrer: "no-referrer",
    headers: {
      accept: "application/json",
      ...(apiKey ? { Authorization: "Bearer " + apiKey } : {}),
    },
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const text = await response.text();

  const json = parseOrElse<DiffOperationResponse>(text, {
    fuzzy_results: [],
    state: [],
  });

  return json;
}
