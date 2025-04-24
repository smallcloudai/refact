import { test, expect } from "@playwright/test";
import { FakeIde } from "../fixtures/FakeIde";

test("fake ide", async ({ page }) => {
  const fakeIde = await FakeIde.initialize(page);
  await page.goto("/");
});
