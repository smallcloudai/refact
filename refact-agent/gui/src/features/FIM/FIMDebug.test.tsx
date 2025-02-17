import { expect, vi, describe, afterEach, beforeEach, test } from "vitest";
import {
  render,
  waitFor,
  postMessage,
  stubResizeObserver,
  cleanup,
} from "../../utils/test-utils";
import { FIMDebug, receive } from ".";
import { STUB } from "../../__fixtures__/fim";
import { Provider } from "react-redux";
import { store } from "../../app/store";

const App = () => {
  return (
    <Provider store={store}>
      <FIMDebug host="web" tabbed={false} />
    </Provider>
  );
};

describe("Fill-in-the-middle Context", () => {
  beforeEach(() => {
    stubResizeObserver();
    vi.spyOn(window, "postMessage").mockImplementation(postMessage);
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  test("render stub data", async () => {
    const app = render(<App />);
    postMessage(receive(STUB));

    await waitFor(() =>
      expect(app.queryByText(/Code Completion Context/i)).not.toBeNull(),
    );
  });
});
