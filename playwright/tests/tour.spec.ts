import { test, expect } from "../fixtures";

test("Tour", async ({ page, loginPage, baseURL, tourPage }) => {
  page.goto("/");

  await tourPage.step1();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await tourPage.step2();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await tourPage.step3();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await tourPage.step4();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await tourPage.step5();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });

  await tourPage.step6();
  await expect(page).toHaveScreenshot();

  await tourPage.step7();
  await expect(page).toHaveScreenshot();

  await tourPage.step8();
  await expect(page).toHaveScreenshot();
});
