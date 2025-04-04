import { test, expect } from "../fixtures";

test("Tour", async ({ page, loginPage, baseURL }) => {
  page.goto("/");
  await expect(page.getByText("Welcome to Refact.ai!")).toBeVisible();

  await page.getByRole("button", { name: "Get Started" }).click();
});
