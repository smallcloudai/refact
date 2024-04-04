import { expect, vi, describe, afterEach, beforeEach, test } from "vitest";
import {
  render,
  waitFor,
  postMessage,
  stubResizeObserver,
  cleanup,
  // screen,
} from "../utils/test-utils";
import { FIMDebug } from "./FIMDebug";
import { ReceiveFIMDebugData, FIM_EVENT_NAMES } from "../events";
import { STUB } from "../__fixtures__/fim";

describe("FIM debug page", () => {
  beforeEach(() => {
    stubResizeObserver();
    vi.spyOn(window, "postMessage").mockImplementation(postMessage);
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  test("render stub data", async () => {
    const app = render(<FIMDebug />);

    const dataMessage: ReceiveFIMDebugData = {
      type: FIM_EVENT_NAMES.DATA_RECEIVE,
      payload: STUB,
    };
    postMessage(dataMessage);

    await waitFor(() => expect(app.queryByText(/FIM debug/i)).not.toBeNull());
  });
});
