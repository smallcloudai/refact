import { beforeAll, afterEach, afterAll, vi } from "vitest";
import {
  stubResizeObserver,
  cleanup,
  stubIntersectionObserver,
} from "./test-utils";
import MatchMediaMock from "vitest-matchmedia-mock";
import React from "react";
const matchMediaMock = new MatchMediaMock();

(globalThis as Record<string, unknown>).__REFACT_LSP_PORT__ = 8001;

beforeAll(() => {
  stubResizeObserver();
  stubIntersectionObserver();
  Element.prototype.scrollIntoView = vi.fn();

  // Mock localStorage for tests
  const localStorageMock: Storage = {
    getItem: vi.fn(() => null),
    setItem: vi.fn(),
    removeItem: vi.fn(),
    clear: vi.fn(),
    key: vi.fn(() => null),
    length: 0,
  };
  global.localStorage = localStorageMock;
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
