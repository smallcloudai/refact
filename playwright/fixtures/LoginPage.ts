import { expect, type Page } from "@playwright/test";

const apiKey = process.env.REFACT_API_KEY ?? "test-api-key";

// Create a mock response that matches GoodPollingResponse type

function parseOrElse<T>(str: string, fallback: T): T {
  try {
    const data = JSON.parse(str);
    return data as T;
  } catch {
    return fallback;
  }
}
const StubResponse = {
  retcode: "OK",
  account: "test@example.com",
  inference_url: "https://inference.smallcloud.ai/",
  inference: "PRO",
  metering_balance: 1000,
  questionnaire: true, // TODO: this disables the survey
  refact_agent_max_request_num: 100,
  refact_agent_request_available: null,
  secret_key: apiKey,
  tooltip_message: "Welcome to Refact!",
  login_message: "You are now logged in",
  "longthink-filters": [],
  "longthink-functions-today": {},
  "longthink-functions-today-v2": {},
};
export class LoginPage {
  constructor(public readonly page: Page) {}

  async doLogin(url?: string, openSurvey = false, screenshots = false) {
    // Set up route interception before navigating;
    await this.mockUserRequest(openSurvey);
    // TODO: hard coded for now
    await this.page.goto(url || "http://localhost:3000/");

    await expect(
      this.page.getByRole("heading", { name: "Login to Refact.ai" })
    ).toBeVisible({ timeout: 10000 });

    await this.page.waitForSelector('button:has-text("Continue with Google")');

    if (screenshots) expect(this.page).toHaveScreenshot();

    await this.page.context().on("page", async (newPage) => {
      const url = new URL(newPage.url());
      expect(url.hostname).toEqual("accounts.google.com");
      expect(url.pathname).toEqual("/v3/signin/identifier");
      await newPage.close();
    });

    await this.page.click('button:has-text("Continue with Google")');

    await this.page.waitForLoadState("networkidle");

    if (screenshots) expect(this.page).toHaveScreenshot();

    await expect(this.page).toHaveURL("http://localhost:3000/");
    // wait for route to have been Called
    await expect(
      this.page.getByRole("heading", { name: "Login to Refact.ai" })
    ).not.toBeVisible({ timeout: 10000 });

    if (screenshots)
      expect(this.page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });
  }

  async doLogout() {
    await this.page.goto("/");
    await this.page.getByRole("button", { name: "menu" }).click();
    await this.page.getByRole("menuitem", { name: "Logout" }).click();
  }

  async mockUserRequest(openSurvey = false) {
    const mockResponse = {
      ...StubResponse,
      questionnaire: !openSurvey,
    };
    await this.page
      .context()
      .route(
        "https://www.smallcloud.ai/v1/streamlined-login-recall-ticket",
        async (route) => {
          // Return our mock response
          await route.fulfill({
            status: 200,
            contentType: "application/json",
            body: JSON.stringify(mockResponse),
          });
        }
      );

    await this.page
      .context()
      .route("https://www.smallcloud.ai/v1/login", async (route) => {
        await route.fulfill({
          status: 200,
          contentType: "application/json",
          body: JSON.stringify(mockResponse),
        });
      });
  }
}
