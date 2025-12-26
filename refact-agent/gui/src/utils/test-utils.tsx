import { PropsWithChildren, ReactElement } from "react";
import { vi } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, RenderOptions } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Theme } from "@radix-ui/themes";
import { Provider } from "react-redux";
import { AppStore, RootState, setUpStore } from "../app/store";
import { TourProvider } from "../features/Tour";
import { AbortControllerProvider } from "../contexts/AbortControllers";
import { v4 as uuidv4 } from "uuid";
import type { ChatThreadRuntime } from "../features/Chat/Thread/types";

// Helper to create a default thread runtime for tests
const createTestThreadRuntime = (): ChatThreadRuntime => {
  return {
    thread: {
      id: uuidv4(),
      messages: [],
      title: "",
      model: "",
      last_user_message_id: "",
      tool_use: "explore",
      new_chat_suggested: { wasSuggested: false },
      boost_reasoning: false,
      automatic_patch: false,
      increase_max_tokens: false,
      include_project_info: true,
      context_tokens_cap: undefined,
    },
    streaming: false,
    waiting_for_response: false,
    prevent_send: false,
    error: null,
    queued_messages: [],
    send_immediately: false,
    attached_images: [],
    confirmation: {
      pause: false,
      pause_reasons: [],
      status: {
        wasInteracted: false,
        confirmationStatus: true,
      },
    },
    queue_size: 0,
  };
};

// Helper to create default chat state with a thread
export const createDefaultChatState = () => {
  const runtime = createTestThreadRuntime();
  return {
    current_thread_id: runtime.thread.id,
    open_thread_ids: [runtime.thread.id],
    threads: { [runtime.thread.id]: runtime },
    system_prompt: {},
    tool_use: "explore" as const,
  };
};

// This type interface extends the default options for render from RTL, as well
// as allows the user to specify other things such as initialState, store.
interface ExtendedRenderOptions
  extends Omit<RenderOptions, "queries" | "wrapper"> {
  preloadedState?: Partial<RootState>;
  store?: AppStore;
}

const customRender = (
  ui: ReactElement,
  options: ExtendedRenderOptions = {},
) => {
  const user = userEvent.setup();
  const {
    preloadedState,
    // Automatically create a store instance if no store was passed in
    store = setUpStore({
      // @ts-expect-error finished
      tour: { type: "finished", step: 0 },
      // Provide default chat state with a thread for tests
      chat: createDefaultChatState(),
      ...preloadedState,
    }),
    ...renderOptions
  } = options;

  const Wrapper = ({ children }: PropsWithChildren) => (
    <Provider store={store}>
      <Theme>
        <TourProvider>
          <AbortControllerProvider>{children}</AbortControllerProvider>
        </TourProvider>
      </Theme>
    </Provider>
  );

  return {
    ...render(ui, {
      wrapper: Wrapper,
      ...renderOptions,
    }),
    store,
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

// export function setUpSystemPromptsForChat(chatId = "") {
//   const systemPromptsMessage: ReceivePrompts = {
//     type: EVENT_NAMES_TO_CHAT.RECEIVE_PROMPTS,
//     payload: {
//       id: chatId,
//       prompts: SYSTEM_PROMPTS,
//     },
//   };
//   postMessage(systemPromptsMessage);
// }

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

export function stubIntersectionObserver() {
  const mockIntersectionObserver = vi.fn();
  mockIntersectionObserver.mockReturnValue({
    observe: () => null,
    unobserve: () => null,
    disconnect: () => null,
  });
  vi.stubGlobal("IntersectionObserver", mockIntersectionObserver);
}
