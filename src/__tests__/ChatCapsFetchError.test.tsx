import { render, waitFor } from "../utils/test-utils";
import { describe, expect, test } from "vitest";
import { HttpResponse, http } from "msw";
import {
  server,
  goodPrompts,
  noTools,
  goodUser,
  goodPing,
  chatLinks,
} from "../utils/mockServer";
import { Chat } from "../features/Chat";

describe("chat caps error", () => {
  test("error detail", async () => {
    const errorMessage =
      "500 Internal Server Error caps fetch failed: failed to open file 'hren'";
    server.use(
      goodPing,
      noTools,
      goodPrompts,
      goodUser,
      chatLinks,
      http.get("http://127.0.0.1:8001/v1/caps", () => {
        return HttpResponse.json(
          {
            detail: errorMessage,
          },
          { status: 500 },
        );
      }),
    );

    const app = render(
      <Chat host="vscode" tabbed={false} backFromChat={() => ({})} />,
    );

    const regex = new RegExp(errorMessage, "i");
    await waitFor(() => {
      expect(app.queryByText(regex)).not.toBeNull();
    });
  });
});
