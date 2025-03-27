import {
  expect,
  vi,
  describe,
  it,
  afterEach,
  beforeEach,
  test,
  beforeAll,
  afterAll,
} from "vitest";
import {
  render,
  waitFor,
  stubResizeObserver,
  // setUpSystemPromptsForChat,
  cleanup,
  screen,
} from "../../utils/test-utils";
import { Chat } from "./Chat";
// import {
//   EVENT_NAMES_TO_CHAT,
//   EVENT_NAMES_FROM_CHAT,
//   RestoreChat,
//   CreateNewChatThread,
//   ChatErrorStreaming,
//   ChatReceiveCapsError,
//   ResponseToChat,
//   ToolCall,
//   ToolResult,
// } from "../events";
import { STUB_CAPS_RESPONSE } from "../../__fixtures__";
// import { useEventBusForChat } from "../hooks";

import { http, HttpResponse } from "msw";

import {
  server,
  goodCaps,
  goodPrompts,
  noTools,
  noCommandPreview,
  noCompletions,
  goodUser,
  goodPing,
  chatLinks,
  telemetryChat,
  telemetryNetwork,
} from "../../utils/mockServer";

const handlers = [
  goodCaps,
  goodPrompts,
  noTools,
  noCommandPreview,
  noCompletions,
  goodUser,
  goodPing,
  chatLinks,
  telemetryChat,
  telemetryNetwork,
];

const goodAgentUsage = {
  _persist: {
    rehydrated: true,
    version: 1,
  },
  agent_max_usage_amount: 20,
  agent_usage: 20,
};

// const handlers = [
//   http.get("http://127.0.0.1:8001/v1/caps", () => {
//     return HttpResponse.json(STUB_CAPS_RESPONSE);
//   }),
//   http.get("http://127.0.0.1:8001/v1/tools", () => {
//     return HttpResponse.json([]);
//   }),
//   http.get("http://127.0.0.1:8001/v1/customization", () => {
//     return HttpResponse.json({ system_prompts: SYSTEM_PROMPTS });
//   }),
//   http.post("http://127.0.0.1:8001/v1/at-command-completion", () => {
//     return HttpResponse.json({
//       completions: [],
//       replace: [0, 0],
//       is_cmd_executable: false,
//     });
//   }),

//   http.post("http://127.0.0.1:8001/v1/at-command-preview", () => {
//     return HttpResponse.json({
//       messages: [],
//     });
//   }),
// ];

// const worker = setupServer(...handlers);

const App: React.FC = () => {
  return <Chat host="web" tabbed={false} backFromChat={() => ({})} />;
};

// MAybe render the chat once and use the new chat button a lot ?
afterEach(() => {
  // server.resetHandlers();
  cleanup();
  // vi.restoreAllMocks();
});

describe("Chat", () => {
  beforeAll(() => {
    // worker.listen();
    stubResizeObserver();
  });

  afterAll(() => {
    // worker.close();
  });

  beforeEach(() => {
    // worker.resetHandlers();
    // stubResizeObserver();
    // vi.spyOn(window, "postMessage").mockImplementation(postMessage);
  });

  // afterEach(() => {
  //   // server.resetHandlers();
  //   cleanup();
  //   // vi.restoreAllMocks();
  // });

  it("should send request to the lsp", async () => {
    const encoder = new TextEncoder();
    server.use(...handlers);
    server.use(
      http.post(
        "http://127.0.0.1:8001/v1/chat",
        () => {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(
                encoder.encode(
                  `data: ${JSON.stringify({
                    content: "hello\n",
                    role: "user",
                    tool_call_id: "",
                    usage: null,
                  })}\n\n`,
                ),
              );

              controller.enqueue(
                encoder.encode(
                  `data: ${JSON.stringify({
                    choices: [
                      {
                        delta: {
                          content: "hello",
                          function_call: null,
                          role: "assistant",
                          tool_calls: null,
                        },
                        finish_reason: null,
                        index: 0,
                        logprobs: null,
                      },
                    ],
                  })}\n\n`,
                ),
              );

              controller.enqueue(
                encoder.encode(
                  `data: ${JSON.stringify({
                    choices: [
                      {
                        delta: {
                          content: " there",
                          function_call: null,
                          role: null,
                          tool_calls: null,
                        },
                        finish_reason: null,
                        index: 0,
                        logprobs: null,
                      },
                    ],
                  })}\n\n`,
                ),
              );

              controller.enqueue(
                encoder.encode(
                  `data: ${JSON.stringify({
                    choices: [
                      {
                        delta: {
                          content: null,
                          function_call: null,
                          role: null,
                          tool_calls: null,
                        },
                        finish_reason: "stop",
                        index: 0,
                        logprobs: null,
                      },
                    ],
                  })}\n\n`,
                ),
              );

              controller.enqueue(
                encoder.encode(`data: ${JSON.stringify(["DONE"])}\n\n`),
              );

              controller.close();
            },
          });

          return new HttpResponse(stream, {
            headers: {
              "Content-Type": "application/json",
              "Transfer-Encoding": "chunked",
            },
          });
        },
        // { once: true }, // TODO: title
      ),
    );

    const { user, ...app } = render(
      <Chat host="vscode" tabbed={false} backFromChat={() => ({})} />,
      {
        preloadedState: {
          pages: [{ name: "chat" }],
          agentUsage: goodAgentUsage,
        },
      },
    );

    const textarea = screen.getByTestId("chat-form-textarea");

    expect(textarea).not.toBeNull();

    const quickButtons = app.getAllByText(/quick/i);

    await user.click(quickButtons[0]);

    await user.type(textarea, "hello");

    await waitFor(() =>
      app.queryByText(STUB_CAPS_RESPONSE.chat_default_model),
    );

    await user.keyboard("{Enter}");

    await waitFor(() => {
      expect(screen.getAllByText("hello there")).not.toBeNull();
    });
  });

  // TODO: when no caps it should not send

  // TODO: skip until history is added
  it.skip("when creating a new chat I can select which model to use", async () => {
    // Missing props in jsdom
    // window.PointerEvent = class PointerEvent extends Event {};
    server.use(
      goodPrompts,
      noCommandPreview,
      noCompletions,
      noTools,
      goodCaps,
      goodPing,
    );
    const chatSpy = vi.fn();
    server.use(
      http.post("http://127.0.0.1:8001/v1/chat", (req) => {
        chatSpy(req);
        return HttpResponse.json({});
      }),
    );

    const { user, ...app } = render(<App />);

    // const userInput = await app.findByText("hello");
    // expect(userInput.textContent).toContain("hello");

    // expect(app.queryByTitle("chat model")).toBeNull();

    // await waitFor(() => expect(app.queryByTitle("chat model")).not.toBeNull(), {
    //   timeout: 1000,
    // });
    await waitFor(() =>
      expect(
        app.queryByText(STUB_CAPS_RESPONSE.chat_default_model),
      ).not.toBeNull(),
    );

    await user.click(app.getByTitle("chat model"));

    app.debug(app.container, 100000);

    await user.click(app.getByRole("option", { name: /test-model/i }));

    await waitFor(() => expect(app.queryByText("test-model")).not.toBeNull());

    const textarea: HTMLTextAreaElement | null =
      app.container.querySelector("textarea");

    expect(textarea).not.toBeNull();
    if (textarea) {
      await user.type(textarea, "hello");
      await user.type(textarea, "{enter}");
    }

    expect(chatSpy).toHaveBeenCalled();
  });

  // TODO: skip until chat can initiated with messages
  // it.skip("retry chat", async () => {
  //   vi.mock("uuid", () => ({ v4: () => "foo" }));
  //   const postMessageSpy = vi.spyOn(window, "postMessage");

  //   let id = "";
  //   const { user, ...app } = render(
  //     <App
  //       setId={(v) => {
  //         id = v;
  //       }}
  //     />,
  //   );

  //   const restoreChatAction: RestoreChat = {
  //     type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
  //     payload: {
  //       id: id,
  //       chat: {
  //         id: "bar",
  //         messages: [
  //           ["user", "hello 👋"],
  //           ["assistant", "hello there"],
  //           ["user", "how are you?"],
  //           ["assistant", "fine"],
  //         ],
  //         title: "hello",
  //         model: "gpt-3.5-turbo",
  //       },
  //     },
  //   };

  //   postMessage(restoreChatAction);

  //   await waitFor(() => expect(app.queryByText("hello 👋")).not.toBeNull());

  //   const retryButton = app.getByText(/hello 👋/);

  //   await user.click(retryButton);

  //   const textarea: HTMLTextAreaElement | null =
  //     app.container.querySelector("textarea");

  //   expect(textarea).not.toBeNull();
  //   if (textarea) {
  //     textarea.setSelectionRange(0, textarea.value.length);
  //     await user.type(textarea, "{Enter}");
  //   }

  //   expect(postMessageSpy).toHaveBeenLastCalledWith(
  //     {
  //       type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
  //       payload: {
  //         id: "bar",
  //         messages: [["user", "hello 👋"]],
  //         title: "hello",
  //         model: "gpt-3.5-turbo",
  //         attach_file: false,
  //         tools: null,
  //       },
  //     },
  //     "*",
  //   );
  // });

  it("chat error streaming", async () => {
    const encoder = new TextEncoder();
    server.use(
      goodPing,
      goodPrompts,
      noCommandPreview,
      goodCaps,
      noCommandPreview,
      noCompletions,
      noTools,
      chatLinks,
      telemetryChat,
      telemetryNetwork,
    );
    server.use(
      http.post(
        "http://127.0.0.1:8001/v1/chat",
        () => {
          const stream = new ReadableStream({
            start(controller) {
              controller.enqueue(
                encoder.encode(
                  `data: ${JSON.stringify({
                    detail: "whoops",
                  })}\n\n`,
                ),
              );
            },
          });
          return new HttpResponse(stream, {
            headers: {
              "Content-Type": "application/json",
              "Transfer-Encoding": "chunked",
            },
          });
        },
        // { once: true }, TODO: title
      ),
    );
    const { user, ...app } = render(<App />, {
      preloadedState: {
        agentUsage: goodAgentUsage,
      },
    });

    const textarea = app.getByTestId("chat-form-textarea");

    expect(textarea).not.toBeNull();

    const quickButtons = app.getAllByText(/quick/i);

    await user.click(quickButtons[0]);

    await user.type(textarea, "hello");

    await user.keyboard("{Enter}");

    await waitFor(() => expect(app.queryByText(/whoops/)).not.toBeNull());
  });

  test.skip("chat with different system prompt", async () => {
    // Missing props in jsdom
    // window.PointerEvent = class PointerEvent extends Event {};
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
    window.HTMLElement.prototype.hasPointerCapture = vi.fn();
    window.HTMLElement.prototype.releasePointerCapture = vi.fn();

    // const postMessageSpy = vi.spyOn(window, "postMessage");
    // const windowSpy = vi.fn();
    // window.addEventListener("message", windowSpy);

    const { user, ...app } = render(<App />);

    // setUpSystemPromptsForChat(id);

    const btn = await waitFor(() => app.getByTitle("default"), {
      timeout: 1000,
    });

    await user.click(btn);

    await user.click(app.getByText(/insert_jokes/i));

    const textarea = app.getByTestId("chat-form-textarea");

    expect(textarea).not.toBeNull();

    await user.type(textarea, "hello");

    await user.keyboard("{Enter}");

    // expect(postMessageSpy).toHaveBeenCalledWith(
    //   {
    //     type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
    //     payload: {
    //       id,
    //       title: "",
    //       model: "",
    //       attach_file: false,
    //       tools: null,
    //       messages: [
    //         ["system", SYSTEM_PROMPTS.insert_jokes.text],
    //         ["user", "hello\n"],
    //       ],
    //     },
    //   },
    //   "*",
    // );
  });

  // test("restore and receive response with use question", async () => {
  //   vi.mock("uuid", () => ({ v4: () => "foo" }));
  //   let id = "";
  //   const app = render(
  //     <App
  //       setId={(v) => {
  //         id = v;
  //       }}
  //     />,
  //   );

  //   const restoreChatAction: RestoreChat = {
  //     type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
  //     payload: {
  //       id,
  //       chat: {
  //         id: "bar",
  //         messages: [
  //           ["user", "/shorter"],
  //           ["assistant", "hello there"],
  //           ["user", "even shorter still"],
  //         ],
  //         title: "hello",
  //         model: "gpt-3.5-turbo",
  //       },
  //     },
  //   };

  //   postMessage(restoreChatAction);

  //   await waitFor(() => expect(app.queryByText("hello there")).not.toBeNull());

  //   const file: ResponseToChat = {
  //     type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
  //     payload: {
  //       id: "bar",
  //       content:
  //         '[{"file_name":"/refact-chat-js/src/services/refact.ts","file_content":"hello","line1":121,"line2":451,"usefulness":100.0}]',
  //       role: "context_file",
  //     },
  //   };

  //   postMessage(file);

  //   const assistant: ResponseToChat = {
  //     type: EVENT_NAMES_TO_CHAT.CHAT_RESPONSE,
  //     payload: {
  //       id: "bar",
  //       role: "user",
  //       content: "even shorter still",
  //     },
  //   };

  //   postMessage(assistant);

  //   postMessage({
  //     type: EVENT_NAMES_TO_CHAT.DONE_STREAMING,
  //     payload: { id: "bar" },
  //   });

  //   await new Promise((r) => setTimeout(r, 500));

  //   const messages = app.getAllByText("even shorter still");
  //   expect(messages.length).toBe(1);

  //   expect(() => app.queryByText("hello there")).not.toBeNull();
  // });

  // test("Chat with functions", async () => {
  //   const postMessageSpy = vi.spyOn(window, "postMessage");

  //   window.HTMLElement.prototype.scrollIntoView = vi.fn();
  //   window.HTMLElement.prototype.hasPointerCapture = vi.fn();
  //   window.HTMLElement.prototype.releasePointerCapture = vi.fn();

  //   let id = "";
  //   const { user, ...app } = render(
  //     <App
  //       setId={(v) => {
  //         id = v;
  //       }}
  //     />,
  //   );

  //   const toolCalls: ToolCall[] = [
  //     {
  //       id,
  //       function: {
  //         name: "cat",
  //         arguments: JSON.stringify({ file: "meow.txt" }),
  //       },
  //       type: "function",
  //       index: 0,
  //     },
  //   ];

  //   const toolResult: ToolResult = {
  //     tool_call_id: "a",
  //     finish_reason: "call_worked",
  //     content: "meow\nmeow\n🐈\n",
  //   };

  //   const restoreChatAction: RestoreChat = {
  //     type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
  //     payload: {
  //       id,
  //       chat: {
  //         id: "bar",
  //         messages: [
  //           ["user", "hello"],
  //           ["assistant", "hello there", toolCalls],
  //           ["tool", toolResult],
  //         ],
  //         title: "hello",
  //         model: "gpt-3.5-turbo",
  //       },
  //     },
  //   };

  //   postMessage(restoreChatAction);

  //   const textarea = app.getByTestId("chat-form-textarea");

  //   expect(textarea).not.toBeNull();

  //   await user.type(textarea, "hello");

  //   await user.keyboard("{Enter}");

  //   expect(postMessageSpy).toHaveBeenCalledWith(
  //     {
  //       type: EVENT_NAMES_FROM_CHAT.ASK_QUESTION,
  //       payload: {
  //         id: "bar",
  //         title: "hello",
  //         model: "gpt-3.5-turbo",
  //         attach_file: false,
  //         tools: null,
  //         messages: [
  //           ["user", "hello"],
  //           ["assistant", "hello there", toolCalls],
  //           ["tool", toolResult],
  //           ["user", "hello\n"],
  //         ],
  //       },
  //     },
  //     "*",
  //   );
  // });

  // test("Prevent send when restored with uncalled tool_calls", async () => {
  //   let id = "";
  //   const app = render(
  //     <App
  //       setId={(v) => {
  //         id = v;
  //       }}
  //     />,
  //   );

  //   const restoreChatAction: RestoreChat = {
  //     type: EVENT_NAMES_TO_CHAT.RESTORE_CHAT,
  //     payload: {
  //       id,
  //       chat: {
  //         id: "bar",
  //         messages: [
  //           ["user", "hello 👋"],
  //           [
  //             "assistant",
  //             "calling tools",
  //             [
  //               {
  //                 function: {
  //                   arguments: '{"file": "foo.txt"}',
  //                   name: "cat",
  //                 },
  //                 index: 0,
  //                 type: "function",
  //                 id: "test",
  //               },
  //             ],
  //           ],
  //         ],
  //         title: "hello",
  //         model: "gpt-3.5-turbo",
  //       },
  //     },
  //   };

  //   postMessage(restoreChatAction);

  //   await waitFor(() => expect(app.queryByText("hello 👋")).not.toBeNull());

  //   const button = app.queryByText(/resume/i);

  //   expect(button).not.toBeNull();
  // });
});
