const TELEMETRY_CHAT_PATH = "/v1/telemetry-chat";

export function sendTelemetryEvent({
  port = 8001,
  scope,
  success,
  error_message,
}: {
  port?: number;
  scope: string;
  success: boolean;
  error_message: string;
}) {
  const url = `http://127.0.0.1:${port}${TELEMETRY_CHAT_PATH}`;
  if (error_message.length > 200) {
    error_message = error_message.substring(0, 200) + "...";
  }
  void fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      scope,
      success,
      error_message,
    }),
  });
}
