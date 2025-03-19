import { beforeAll, afterEach, afterAll, vi } from "vitest";
import { stubResizeObserver, cleanup } from "./test-utils";
import MatchMediaMock from "vitest-matchmedia-mock";

const matchMediaMock = new MatchMediaMock();

beforeAll(() => {
  stubResizeObserver();
  Element.prototype.scrollIntoView = vi.fn();
});

afterEach(() => {
  cleanup();
});

afterAll(() => {
  matchMediaMock.destroy();
});
