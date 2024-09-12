type FullpathResponse = {
  fullpath: string;
  is_directory: boolean;
};

function isFullpathResponse(x: unknown): x is FullpathResponse {
  if (typeof x !== "object" || x === null) {
    return false;
  }
  if (!("fullpath" in x) || !("is_directory" in x)) {
    return false;
  }
  if (typeof x.fullpath !== "string") {
    return false;
  }
  if (typeof x.is_directory !== "boolean") {
    return false;
  }
  return true;
}

export async function getFullpath(
  path: string,
  port: number,
): Promise<string | null> {
  const url = `http://127.0.0.1:${port}/v1/fullpath`;

  const response = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ path }),
  });
  const result: unknown = await response.json();
  if (!isFullpathResponse(result)) {
    return null;
  }
  if (result.is_directory) {
    return null;
  }
  return result.fullpath;
}
