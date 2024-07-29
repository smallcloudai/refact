import { ReactElement } from "react";
import { vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, RenderOptions } from "@testing-library/react";
import userEvent, { UserEvent } from "@testing-library/user-event";
import { Theme } from "@radix-ui/themes";
import { EVENT_NAMES_TO_CHAT, ReceivePrompts } from "../events";
import { SYSTEM_PROMPTS } from "../__fixtures__";

const customRender = (
  ui: ReactElement,
  options?: Omit<RenderOptions, "wrapper">,
): ReturnType<typeof render> & { user: UserEvent } => {
  const user = userEvent.setup();
  return {
    ...render(ui, { wrapper: Theme, ...options }),
    user,
  };
};

// eslint-disable-next-line react-refresh/only-export-components
export * from "@testing-library/react";

export { customRender as render };

export function postMessage(data: unknown) {
  return window.dispatchEvent(
    new MessageEvent("message", {
      source: window,
      origin: window.location.origin,
      data,
    }),
  );
}

// export function setUpCapsForChat(chatId = "") {
//   postMessage({
//     type: EVENT_NAMES_TO_CHAT.RECEIVE_CAPS,
//     payload: {
//       id: chatId,
//       caps: STUB_CAPS_RESPONSE,
//     },
//   });
// }

export function setUpSystemPromptsForChat(chatId = "") {
  const systemPromptsMessage: ReceivePrompts = {
    type: EVENT_NAMES_TO_CHAT.RECEIVE_PROMPTS,
    payload: {
      id: chatId,
      prompts: SYSTEM_PROMPTS,
    },
  };
  postMessage(systemPromptsMessage);
}

export function stubResizeObserver() {
  const ResizeObserverMock = vi.fn(() => ({
    observe: vi.fn(),
    unobserve: vi.fn(),
    disconnect: vi.fn(),
  }));

  // Stub the global ResizeObserver
  vi.stubGlobal("ResizeObserver", ResizeObserverMock);
}

/**
 * repeat use with describe.each or test.each to find flaky tests
 * @param n
 * @returns an array of n numbers
 *
 */
export const repeat = (n: number) =>
  Array.from({ length: n }).map((_d, i) => i + 1);
