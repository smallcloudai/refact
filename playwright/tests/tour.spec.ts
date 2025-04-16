import { test, expect } from "../fixtures";

test("Tour", async ({ page, loginPage, baseURL }) => {
  page.goto("/");
  // TODO: turn this into a fixture
  await expect(page.getByText("Welcome to Refact.ai!")).toBeVisible();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await page.getByRole("button", { name: "Get Started" }).click();
  await expect(page.getByText("Agent can accomplish tasks")).toBeVisible();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await page.getByText("next").click();
  await expect(page.getByText("Integrations", { exact: true })).toBeVisible();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await page.getByText("next").click();
  await expect(page.getByText("Chat modes / models")).toBeVisible();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await page.getByText("next").click();
  await expect(page.getByText("Difference in Quick / Explore")).toBeVisible();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await page.getByText("next").click();
  await expect(page.getByText("Code completion")).toBeVisible();
  await expect(page).toHaveScreenshot();

  await page.getByText("next").click();
  await expect(
    page.getByText("Your Refact product tour is finished!")
  ).toBeVisible();
  await expect(page).toHaveScreenshot();

  await page.getByRole("button", { name: "Ready to use" }).click();
  await expect(page).toHaveScreenshot();
});
