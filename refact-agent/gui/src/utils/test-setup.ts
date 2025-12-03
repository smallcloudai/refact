import { beforeAll, afterEach, afterAll, vi } from "vitest";
import {
  stubResizeObserver,
  cleanup,
  stubIntersectionObserver,
} from "./test-utils";
import MatchMediaMock from "vitest-matchmedia-mock";
import React from "react";
const matchMediaMock = new MatchMediaMock();

beforeAll(() => {
  stubResizeObserver();
  stubIntersectionObserver();
  Element.prototype.scrollIntoView = vi.fn();

  // Mock localStorage for tests
  const localStorageMock = {
    getItem: vi.fn(),
    setItem: vi.fn(),
    removeItem: vi.fn(),
    clear: vi.fn(),
  };
  global.localStorage = localStorageMock as Storage;
});

afterEach(() => {
  cleanup();
});

afterAll(() => {
  matchMediaMock.destroy();
});

vi.mock("lottie-react", () => {
  return {
    default: vi.fn(),
    useLottie: vi.fn(() => {
      return {
        View: React.createElement("div"),
        playSegments: vi.fn(),
      };
    }),
  };
});
