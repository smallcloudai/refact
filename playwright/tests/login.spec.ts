import { test, expect } from "@playwright/test";

test.describe("Login functionality", () => {
  test("successful login with network interception", async ({ page }) => {
    // 1. Set up network interception
    // We'll listen for the authentication API call
    let authRequestReceived = false;
    let authResponseReceived = false;
    
    // Intercept the auth request
    await page.route("**/api/auth/login", async (route) => {
      // Record that we've seen the request
      authRequestReceived = true;
      
      // Inspect the request if needed
      const request = route.request();
      console.log(`Intercepted ${request.method()} request to ${request.url()}`);
      
      // Let the request continue normally
      await route.continue();
    });
    
    // Intercept the auth response
    await page.on("response", async (response) => {
      if (response.url().includes("/api/auth/login")) {
        authResponseReceived = true;
        
        // Check response status
        expect(response.status()).toBe(200);
        
        // Optionally inspect the response body
        try {
          const responseBody = await response.json();
          console.log("Auth response:", responseBody);
          // You can add assertions on the response body if needed
        } catch (error) {
          console.error("Failed to parse response JSON:", error);
        }
      }
    });
    
    // 2. Navigate to the login page (assuming it's the root path or has a login endpoint)
    await page.goto("/");
    
    // 3. Assert the login page is displayed correctly
    await expect(page.getByRole("heading", { name: "Login" })).toBeVisible();
    
    // 4. Fill in the login form
    await page.getByLabel("Email").fill("test@example.com");
    await page.getByLabel("Password").fill("password123");
    
    // 5. Submit the form and wait for navigation
    await Promise.all([
      page.waitForResponse("**/api/auth/login"),
      page.getByRole("button", { name: "Login" }).click()
    ]);
    
    // 6. Verify network interception worked
    expect(authRequestReceived).toBeTruthy();
    expect(authResponseReceived).toBeTruthy();
    
    // 7. Verify user is redirected to the dashboard or home page after login
    // This might need to be adjusted based on your app's actual behavior
    await expect(page).toHaveURL(/dashboard|home/);
    
    // 8. Verify user is logged in (e.g., by checking for certain UI elements)
    await expect(page.getByText("Welcome")).toBeVisible();
    
    // 9. Optionally verify a token is stored in localStorage
    const hasToken = await page.evaluate(() => {
      return !!localStorage.getItem("auth_token");
    });
    expect(hasToken).toBeTruthy();
  });
  
  test("failed login with incorrect credentials", async ({ page }) => {
    // 1. Set up network interception for the failed login attempt
    let failedAuthResponse = null;
    
    // Listen for the failed auth response
    await page.on("response", async (response) => {
      if (response.url().includes("/api/auth/login") && response.status() !== 200) {
        failedAuthResponse = response;
      }
    });
    
    // 2. Navigate to the login page
    await page.goto("/");
    
    // 3. Fill in incorrect credentials
    await page.getByLabel("Email").fill("wrong@example.com");
    await page.getByLabel("Password").fill("wrongpassword");
    
    // 4. Submit the form and expect an error response
    await page.getByRole("button", { name: "Login" }).click();
    
    // 5. Wait for error message to appear
    await expect(page.getByText("Invalid email or password")).toBeVisible();
    
    // 6. Verify we're still on the login page
    await expect(page).toHaveURL("/");
    
    // 7. Verify no auth token was set
    const hasToken = await page.evaluate(() => {
      return !!localStorage.getItem("auth_token");
    });
    expect(hasToken).toBeFalsy();
  });
});
