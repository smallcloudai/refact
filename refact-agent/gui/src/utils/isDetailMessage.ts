export type RTKResponseErrorWithDetailMessage = {
  error: {
    data: {
      detail: string;
    };
  };
};

export function isRTKResponseErrorWithDetailMessage(
  json: unknown,
): json is RTKResponseErrorWithDetailMessage {
  const result =
    json &&
    typeof json === "object" &&
    "error" in json &&
    json.error &&
    typeof json.error === "object" &&
    "data" in json.error &&
    json.error.data &&
    typeof json.error.data === "object" &&
    "detail" in json.error.data &&
    typeof json.error.data.detail === "string"
      ? true
      : false;

  return result;
}
