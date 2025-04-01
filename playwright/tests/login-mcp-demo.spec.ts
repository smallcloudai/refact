import { test, expect } from "@playwright/test";

test("Login with network interception", async ({ page }) => {
  // Start network interception
  await page.route("**/authenticate", async (route) => {
    // Log the original request for debugging
    const request = route.request();
    console.log("Intercepted request:", {
      url: request.url(),
      method: request.method(),
      postData: request.postDataJSON(),
      headers: request.headers()
    });
    
    // Continue with the original request
    await route.continue();
  });
  
  // Also monitor responses
  page.on("response", async (response) => {
    if (response.url().includes("/authenticate")) {
      console.log("Response intercepted:", {
        status: response.status(),
        statusText: response.statusText()
      });
      
      try {
        // Try to capture the response body
        const body = await response.json();
        console.log("Response body:", body);
      } catch (e) {
        console.log("Could not parse response as JSON");
      }
    }
  });

  // Navigate to the login page
  await page.goto("https://the-internet.herokuapp.com/login");
  
  // Fill in the login form
  await page.locator("input#username").fill("tomsmith");
  await page.locator("input#password").fill("SuperSecretPassword!");
  
  // Submit the form and wait for navigation
  await Promise.all([
    page.waitForNavigation(),
    page.locator("button[type='submit']").click()
  ]);
  
  // Verify successful login
  await expect(page).toHaveURL(/\/secure$/);
  await expect(page.locator("h4.subheader")).toContainText("Welcome to the Secure Area");
  
  // Test failed login scenario
  await page.goto("https://the-internet.herokuapp.com/login");
  await page.locator("input#username").fill("wronguser");
  await page.locator("input#password").fill("wrongpassword");
  
  // Submit and check for error message
  await page.locator("button[type='submit']").click();
  
  // Error message should be displayed
  await expect(page.locator("div.flash.error")).toBeVisible();
  await expect(page.locator("div.flash.error")).toContainText("Your username is invalid!");
});

// Test login with custom response mocking
test("Login with mocked response", async ({ page }) => {
  // Mock the authentication response
  await page.route("**/authenticate", async (route) => {
    // Instead of letting the request go through, we'll respond with our own data
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        status: 'success',
        message: 'User logged in successfully',
        token: 'fake-jwt-token-for-testing',
        user: {
          id: 1,
          username: 'testuser',
          email: 'test@example.com'
        }
      })
    });
    
    console.log("Responded with mocked authentication data");
  });
  
  // Navigate to login page
  await page.goto("https://the-internet.herokuapp.com/login");
  
  // Fill the form
  await page.locator("input#username").fill("tomsmith");
  await page.locator("input#password").fill("SuperSecretPassword!");
  
  // Submit the form
  await page.locator("button[type='submit']").click();
  
  // Here you would check if your app behaves correctly with the mocked response
  // Since we're using a demo site, this specific mock won't work as expected
  // But this demonstrates how to create a mock response
});
