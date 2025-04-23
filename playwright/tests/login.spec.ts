import { test, expect } from "@playwright/test";
import { LoginPage } from "../fixtures/LoginPage";
import { FakeIde } from "fixtures/FakeIde";

test.use({ storageState: { cookies: [], origins: [] } });
test("login through google with API stub", async ({ page, baseURL }) => {
  const fakeIde = await FakeIde.initialize(page);
  const loginPage = new LoginPage(page);
  await loginPage.doLogin(baseURL, false, true);
  await expect(
    page.getByRole("heading", { name: "Login to Refact.ai" })
  ).not.toBeVisible({ timeout: 10000 });
});
