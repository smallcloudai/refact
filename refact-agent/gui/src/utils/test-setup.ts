import { beforeAll, afterEach, afterAll } from "vitest";
import { stubResizeObserver, cleanup } from "./test-utils";
import MatchMediaMock from "vitest-matchmedia-mock";

const matchMediaMock = new MatchMediaMock();

beforeAll(() => {
  stubResizeObserver();
});

afterEach(() => {
  cleanup();
});

afterAll(() => {
  matchMediaMock.destroy();
});
