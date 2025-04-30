import { test, expect } from "../fixtures";

test("login through google with API stub", async ({
  page,
  baseURL,
  auth,
  fakeIde,
}) => {
  await auth.doLogin(baseURL, false, true);
  await expect(
    page.getByRole("heading", { name: "Login to Refact.ai" })
  ).not.toBeVisible({ timeout: 10000 });
});
