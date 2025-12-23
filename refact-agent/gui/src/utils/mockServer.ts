import { afterAll, afterEach, beforeAll } from "vitest";
import { setupServer } from "msw/node";
import type { Store } from "../app/store";
import {
  capsApi,
  statisticsApi,
  promptsApi,
  toolsApi,
  commandsApi,
  pingApi,
} from "../services/refact";

export * from "../__fixtures__/msw";

export const resetApi = (store: Store) => {
  store.dispatch(capsApi.util.resetApiState());
  store.dispatch(statisticsApi.util.resetApiState());
  store.dispatch(promptsApi.util.resetApiState());
  store.dispatch(toolsApi.util.resetApiState());
  store.dispatch(commandsApi.util.resetApiState());
  store.dispatch(pingApi.util.resetApiState());
};
export const server = setupServer();

beforeAll(() => {
  // Enable the mocking in tests.
  server.listen({ onUnhandledRequest: "error" });
});

afterEach(() => {
  // Reset any runtime handlers tests may use.
  server.resetHandlers();
});

afterAll(() => {
  // Clean up once the tests are done.
  server.close();
});
