import { getApiKey, parseOrElse } from "../../utils";
import { DIFF_APPLY_URL, DIFF_UNDO_URL, DEFF_APPLIED_CHUNKS } from "./consts";
import { DiffChunk } from "./types";

export interface DiffPost {
  chat_id: string;
  message_id: string;
  content: DiffChunk[];
}

export interface DiffAppliedStatePost {
  chat_id: string;
  message_id: string;
}

export interface DiffAppliedStateResponse {
  applied_chunks: number[];
}

export async function checkDiff(body: DiffAppliedStatePost, lspUrl?: string) {
  const addr = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${DEFF_APPLIED_CHUNKS}`
    : DEFF_APPLIED_CHUNKS;

  const apiKey = getApiKey();

  const response = await fetch(addr, {
    method: "POST",
    body: JSON.stringify(body),
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
    applied_chunks: [],
  });

  return json;
}

interface DiffResponseItem {
  chunk_id: number;
  fuzzy_n_used: number;
}

export async function applyDiff(
  body: DiffPost,
  lspUrl?: string,
): Promise<DiffResponseItem[]> {
  const addr = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${DIFF_APPLY_URL}`
    : DIFF_APPLY_URL;

  const apiKey = getApiKey();

  const response = await fetch(addr, {
    method: "POST",
    body: JSON.stringify(body),
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

  const json = parseOrElse<DiffResponseItem[]>(text, []);

  return json;
}

export async function undoDiff(body: DiffPost, lspUrl?: string) {
  const addr = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${DIFF_UNDO_URL}`
    : DIFF_UNDO_URL;

  const apiKey = getApiKey();

  const response = await fetch(addr, {
    method: "POST",
    body: JSON.stringify(body),
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

  const json = parseOrElse<DiffResponseItem[]>(text, []);

  return json;
}
