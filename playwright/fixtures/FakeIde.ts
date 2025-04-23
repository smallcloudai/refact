import { Page } from "@playwright/test";
import {
  setSelectedSnippet,
  setCurrentProjectInfo,
  setFileInfo,
  setInputValue,
  updateConfig,
  fim,
  ideToolCallResponse,
  ideAttachFileToChat,
  newChatAction,
  type Snippet,
  type CurrentProjectInfo,
  type InputActionPayload,
  type Config,
  type FimDebugData,
  type ToolCallResponsePayload,
  type FileInfo,
} from "../../refact-agent/gui/src/events";

declare global {
  interface Window {
    logMessage: typeof window.postMessage;
  }
}

type EventsToIde =
  | ReturnType<typeof setSelectedSnippet>
  | ReturnType<typeof setCurrentProjectInfo>
  | ReturnType<typeof setFileInfo>
  | ReturnType<typeof setInputValue>
  | ReturnType<typeof updateConfig>
  | ReturnType<typeof fim.receive>
  | ReturnType<typeof fim.error>
  | ReturnType<typeof ideToolCallResponse>
  | ReturnType<typeof ideAttachFileToChat>
  | ReturnType<typeof newChatAction>;

export class FakeIde {
  messages: Parameters<typeof window.logMessage>[] = [];

  private constructor(public readonly page: Page) {}

  public static async setUp(
    page: Page,
    host: "vscode" | "jetbrains" | "ide" = "vscode"
  ) {
    // TODO: initial state
    const fakeIde = new FakeIde(page);
    // TODO: mock event bus https://playwright.dev/docs/mock-browser-apis

    await fakeIde.mockMessageBus(host);
  }

  private logMessage(...args: Parameters<typeof window.logMessage>) {
    this.messages.push(args);
  }

  private async mockMessageBus(
    host: "vscode" | "jetbrains" | "ide" = "vscode"
  ) {
    if (host === "vscode") {
      return this.page.exposeFunction("acquireVsCodeApi", () => ({
        postMessage: this.logMessage,
      }));
    }

    if (host === "jetbrains") {
      return this.page.exposeFunction("postIntellijMessage", this.logMessage);
    }

    await this.page.exposeFunction("logMessage", this.logMessage);
    return await this.page.addInitScript(() => {
      const originalPostMessage = window.postMessage;

      window.postMessage = (...args: Parameters<typeof window.postMessage>) => {
        window.logMessage(...args);
        originalPostMessage(...args);
      };
    });
  }

  async dispatch(message: EventsToIde) {
    return this.page.locator("window").dispatchEvent("message", message);
  }

  async clearMessages() {
    this.messages = [];
    return this;
  }

  async addFileInfo(fileInfo: FileInfo) {
    const action = setFileInfo(fileInfo);
    await this.dispatch(action);
    return this;
  }

  async setSelectedSnippet(selectedSnippet: Snippet) {
    const action = setSelectedSnippet(selectedSnippet);
    await this.dispatch(action);
    return this;
  }

  async setCurrentProjectInfo(currentProjectInfo: CurrentProjectInfo) {
    const action = setCurrentProjectInfo(currentProjectInfo);
    await this.dispatch(action);
    return this;
  }

  async setInputValue(inputValue: InputActionPayload) {
    const action = setInputValue(inputValue);
    await this.dispatch(action);
    return this;
  }

  async updateConfig(config: Config) {
    const action = updateConfig(config);
    await this.dispatch(action);
    return this;
  }

  async sendFimData(data: FimDebugData) {
    const action = fim.receive(data);
    await this.dispatch(action);
    return this;
  }

  async sendFimError(error: string) {
    const action = fim.error(error);
    await this.dispatch(action);
    return this;
  }

  async sendToolCallResponse(res: ToolCallResponsePayload) {
    const action = ideToolCallResponse(res);
    await this.dispatch(action);
    return this;
  }

  async sendAttachFileToChat(fileName: string) {
    const action = ideAttachFileToChat(fileName);
    await this.dispatch(action);
    return this;
  }

  async newChat() {
    const action = newChatAction();
    await this.dispatch(action);
    return this;
  }
}
