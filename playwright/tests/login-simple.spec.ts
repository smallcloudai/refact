import { test, expect } from "@playwright/test";

/**
 * Simplified login test
 */

test("successful login basic test", async ({ page }) => {
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
});
