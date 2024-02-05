import { beforeAll, afterEach } from "vitest";
import { stubResizeObserver, cleanup } from "./test-utils";

beforeAll(() => {
  stubResizeObserver();
});

afterEach(() => {
  cleanup();
});
