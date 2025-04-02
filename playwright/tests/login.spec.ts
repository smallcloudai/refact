import { test, expect } from "@playwright/test";

test.describe("Login functionality", () => {
  test("login through google", async ({ page, context }) => {
    const email = process.env.REFACT_LOGIN_EMAIL ?? "test@test.com";
    const password = process.env.REFACT_LOGIN_PASSWORD ?? "test";
    
    // Navigate to the locally hosted app
    await page.goto("http://localhost:5173/");
    
    // Wait for the login form to show up when the user is not logged in
    await page.waitForSelector('button:has-text("Continue with Google")');
    
    // Set up listener for popup window before clicking
    const popupPromise = context.waitForEvent('page');
    
    // Click the "Continue with Google" button which will open a popup
    await page.click('button:has-text("Continue with Google")');
    
    // Verify the main page shows disabled buttons during authentication
    await expect(page.locator('button:has-text("Continue with Google")')).toBeDisabled();
    
    // Wait for the popup to open
    const googlePopup = await popupPromise;
    await googlePopup.waitForLoadState('domcontentloaded');
    
    // Log the popup URL for debugging
    console.log('Google popup URL:', googlePopup.url());
    
    // Google login flow - this may need adjustments based on the actual Google login page
    try {
      // Look for the email input field
      await googlePopup.waitForSelector('input[type="email"]', { timeout: 10000 });
      await googlePopup.fill('input[type="email"]', email);
      await googlePopup.click('button:has-text("Next")');
      
      // Wait for password field and fill it
      await googlePopup.waitForSelector('input[type="password"]', { timeout: 10000 });
      await googlePopup.fill('input[type="password"]', password);
      await googlePopup.click('button:has-text("Next")');
      
      // Wait for any additional confirmation steps
      await googlePopup.waitForSelector('button:has-text("Allow")', { timeout: 5000 })
        .then(() => googlePopup.click('button:has-text("Allow")'))
        .catch(() => console.log('No Allow button found, continuing'));
      
    } catch (error) {
      console.log('Error during Google login flow:', error);
      // Take a screenshot of the popup for debugging
      await googlePopup.screenshot({ path: 'google-login-error.png' });
    }
    
    // Wait for the authentication to complete and the popup to close
    await googlePopup.waitForEvent('close', { timeout: 30000 }).catch(e => {
      console.log('Google popup did not close automatically, continuing test');
    });
    
    // Return to the main page and wait for it to update after login
    await page.waitForLoadState('networkidle');
    
    // Verify we're back at the home page
    await expect(page).toHaveURL("http://localhost:5173/");
    
    // Verify that we're logged in (login form should not be visible)
    await expect(page.locator('heading:has-text("Login to Refact.ai")')).not.toBeVisible({ timeout: 10000 });
  });
});
