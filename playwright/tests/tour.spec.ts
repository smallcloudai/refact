import { test, expect } from "../fixtures";
import { LoginPage } from "../fixtures/LoginPage";

test.use({ storageState: { cookies: [], origins: [] } });
test("Tour", async ({ page, baseURL, tourPage }) => {
  const loginPage = new LoginPage(page);
  await loginPage.doLogin(baseURL, false, false);

  await tourPage.step1();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step2();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step3();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step4();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step5();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step6();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step7();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step8();
  await expect(page).toHaveScreenshot();
});
