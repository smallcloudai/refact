import { Page } from "@playwright/test";
import * as ideEvents from "../../refact-agent/gui/src/events";

declare global {
  interface Window {
    logMessage: typeof window.postMessage;
  }
}

class FakeIde {
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

  async dispatch(message: string) {
    return this.page.locator("window").dispatchEvent("message", message);
  }
}
