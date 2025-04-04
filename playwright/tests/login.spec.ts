import { test, expect } from "@playwright/test";

test("login through google with API stub", async ({ page, context }) => {
  // Mock API key from environment variable
  const apiKey = process.env.REFACT_API_KEY ?? "test-api-key";

  // Create a mock response that matches GoodPollingResponse type
  const mockResponse = {
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

  // Set up route interception before navigating
  await context.route(
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

  // Navigate to the locally hosted app
  await page.goto("http://localhost:5173/");

  await expect(
    page.getByRole("heading", { name: "Login to Refact.ai" })
  ).toBeVisible({ timeout: 10000 });

  // Wait for the login form to show up when the user is not logged in
  await page.waitForSelector('button:has-text("Continue with Google")');

  // Click the "Continue with Google" button
  await page.click('button:has-text("Continue with Google")');

  // Since we're stubbing the API, we don't need to handle the Google popup
  // The app should receive our mock response and proceed with login

  // Wait for the authentication to complete
  await page.waitForLoadState("networkidle");

  // Verify we're back at the home page
  await expect(page).toHaveURL("http://localhost:5173/");

  // Verify that we're logged in (login form should not be visible)
  await expect(
    page.getByRole("heading", { name: "Login to Refact.ai" })
  ).not.toBeVisible({ timeout: 10000 });

  // Additional verification that the API key was properly set
  // This depends on how your app shows the logged-in state
  // You might need to adjust this based on your UI

  // Example: Check if user menu or profile is visible
  // await expect(page.locator('button:has-text("User Profile")')).toBeVisible();
});
