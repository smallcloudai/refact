import { test, expect, Page } from "@playwright/test";

/**
 * Comprehensive login testing suite with network interception
 * This demonstrates both monitoring network traffic and mocking responses
 */

// Helper function to set up request and response monitoring
async function setupNetworkMonitoring(page: Page) {
  // Array to collect request and response data
  const networkLog: Array<{
    type: "request" | "response";
    url: string;
    method?: string;
    status?: number;
    postData?: any;
    responseBody?: any;
  }> = [];

  // Monitor all requests
  await page.route("**/*", async (route) => {
    const request = route.request();

    // Only log and inspect login-related requests
    if (
      request.url().includes("/login") ||
      request.url().includes("/authenticate")
    ) {
      try {
        // Log request details
        const entry = {
          type: "request" as const,
          url: request.url(),
          method: request.method(),
          postData:
            request.method() === "POST"
              ? (await request.postDataBuffer()?.toString()) || null
              : null,
        };

        networkLog.push(entry);
        console.log("Request intercepted:", entry);
      } catch (error) {
        console.error("Error processing request:", error);
      }
    }

    // Continue with the original request
    await route.continue();
  });

  // Monitor all responses
  page.on("response", async (response) => {
    const request = response.request();

    // Only log and inspect login-related responses
    if (
      response.url().includes("/login") ||
      response.url().includes("/authenticate")
    ) {
      try {
        // Create response entry
        const entry = {
          type: "response" as const,
          url: response.url(),
          status: response.status(),
          responseBody: null as any,
        };

        // Try to get response body based on content type
        const contentType = response.headers()["content-type"] || "";

        if (contentType.includes("application/json")) {
          try {
            entry.responseBody = await response.json();
          } catch (e) {
            console.log("Failed to parse JSON response");
          }
        } else if (contentType.includes("text/")) {
          try {
            entry.responseBody = await response.text();
          } catch (e) {
            console.log("Failed to get response text");
          }
        }

        networkLog.push(entry);
        console.log("Response intercepted:", entry);
      } catch (error) {
        console.error("Error processing response:", error);
      }
    }
  });

  return networkLog;
}

test.describe("Login functionality with network interception", () => {
  test("successful login with network monitoring", async ({ page }) => {
    // Set up network monitoring
    const networkLog = await setupNetworkMonitoring(page);

    // Navigate to the login page
    await page.goto("https://the-internet.herokuapp.com/login");

    // Verify the login page loaded correctly
    await expect(page.locator("h2")).toHaveText("Login Page");

    // Fill in the login form with valid credentials
    await page.locator("#username").fill("tomsmith");
    await page.locator("#password").fill("SuperSecretPassword!");

    // Submit the login form and wait for navigation
    await Promise.all([
      page.waitForNavigation(),
      page.locator('button[type="submit"]').click(),
    ]);

    // Verify successful login
    await expect(page).toHaveURL(/\/secure$/);
    await expect(page.locator(".flash.success")).toBeVisible();
    await expect(page.locator("h2")).toHaveText("Secure Area");

    // Verify at least one request and response were captured
    expect(
      networkLog.filter((entry) => entry.type === "request").length
    ).toBeGreaterThan(0);
    expect(
      networkLog.filter((entry) => entry.type === "response").length
    ).toBeGreaterThan(0);

    // Log network entries for debugging
    console.log("Network log entries:", networkLog);
  });

  test("failed login with network monitoring", async ({ page }) => {
    // Set up network monitoring
    const networkLog = await setupNetworkMonitoring(page);

    // Navigate to the login page
    await page.goto("https://the-internet.herokuapp.com/login");

    // Fill in the login form with invalid credentials
    await page.locator("#username").fill("wronguser");
    await page.locator("#password").fill("wrongpassword");

    // Submit the form - no navigation expected since login will fail
    await page.locator('button[type="submit"]').click();

    // Wait for error message to appear
    await expect(page.locator(".flash.error")).toBeVisible();

    // Verify we're still on the login page
    await expect(page).toHaveURL(/\/login$/);

    // Verify error message content
    await expect(page.locator(".flash.error")).toContainText(
      "Your username is invalid"
    );

    // Verify network traffic was captured
    expect(
      networkLog.some(
        (entry) =>
          entry.type === "request" &&
          (entry.url.includes("/login") || entry.url.includes("/authenticate"))
      )
    ).toBeTruthy();

    // Log network entries for debugging
    console.log("Network log entries for failed login:", networkLog);
  });

  test("login with mocked success response", async ({ page }) => {
    // Before navigating, set up our response mock
    await page.route("**/login", async (route) => {
      // Check if this is the form submission POST request
      const request = route.request();
      if (request.method() === "POST") {
        // Instead of sending the actual request, respond with a mock success
        await route.fulfill({
          status: 200,
          contentType: "text/html",
          body: `
            <!DOCTYPE html>
            <html>
            <head>
              <title>Secure Area - Mock Response</title>
            </head>
            <body>
              <div class="flash success">
                You logged in successfully with a mocked response!
              </div>
              <h2>Secure Area (Mocked)</h2>
              <h4>Welcome to the mocked secure area</h4>
              <a href="/logout">Logout</a>
            </body>
            </html>
          `,
        });

        console.log("Responded with mocked success page");
      } else {
        // For GET requests, let them through normally
        await route.continue();
      }
    });

    // Navigate to login page
    await page.goto("https://the-internet.herokuapp.com/login");

    // Fill out the form
    await page.locator("#username").fill("mockuser");
    await page.locator("#password").fill("mockpassword");

    // Submit the form
    await page.locator('button[type="submit"]').click();

    // Verify our mock response was used
    await expect(page.locator(".flash.success")).toContainText(
      "mocked response"
    );
    await expect(page.locator("h2")).toContainText("Mocked");
  });

  test("login with mocked error response", async ({ page }) => {
    // Set up our response mock for the form submission
    await page.route("**/login", async (route) => {
      const request = route.request();
      if (request.method() === "POST") {
        // Mock an error response
        await route.fulfill({
          status: 401, // Unauthorized
          contentType: "text/html",
          body: `
            <!DOCTYPE html>
            <html>
            <head>
              <title>Login Page - Error</title>
            </head>
            <body>
              <div class="flash error">
                Custom mock error: Account locked after too many attempts.
              </div>
              <h2>Login Page</h2>
              <form id="login">
                <input id="username" />
                <input id="password" type="password" />
                <button type="submit">Login</button>
              </form>
            </body>
            </html>
          `,
        });

        console.log("Responded with mocked error page");
      } else {
        // For GET requests, let them through
        await route.continue();
      }
    });

    // Navigate to login page
    await page.goto("https://the-internet.herokuapp.com/login");

    // Fill out the form
    await page.locator("#username").fill("anyuser");
    await page.locator("#password").fill("anypassword");

    // Submit the form
    await page.locator('button[type="submit"]').click();

    // Verify our custom error message
    await expect(page.locator(".flash.error")).toContainText("Account locked");
  });

  test("simulate network error during login", async ({ page }) => {
    // Set up network monitoring
    const networkLog = await setupNetworkMonitoring(page);

    // Set up request abort to simulate network failure
    await page.route("**/login", (route) => {
      const request = route.request();
      if (request.method() === "POST") {
        // Abort the request to simulate network failure
        route.abort("failed");
        console.log("Login request aborted to simulate network failure");
      } else {
        // Let GET requests through normally
        route.continue();
      }
    });

    // Navigate to login page
    await page.goto("https://the-internet.herokuapp.com/login");

    // Fill in the login form
    await page.locator("#username").fill("tomsmith");
    await page.locator("#password").fill("SuperSecretPassword!");

    // Try to submit the form
    await page.locator('button[type="submit"]').click();

    // In a real application, you would now test how the UI handles this network error
    // For example, you might expect an error message to appear

    // Wait a moment for any client-side error handling to execute
    await page.waitForTimeout(1000);

    // We're still on the login page
    await expect(page).toHaveURL(/\/login$/);

    // Log what happened
    console.log("Network log after simulated failure:", networkLog);
  });
});
