import { FakeIde } from "fixtures/FakeIde";
import { test, expect } from "../fixtures";

// test.use({ storageState: { cookies: [], origins: [] } });
test("login through google with API stub", async ({ page, baseURL, auth }) => {
  // const loginPage = new LoginPage(page);
  // await loginPage.doLogin(baseURL, false, true);
  await auth.doLogout();
  await auth.doLogin(baseURL, false, true);
  await expect(
    page.getByRole("heading", { name: "Login to Refact.ai" })
  ).not.toBeVisible({ timeout: 10000 });
});
