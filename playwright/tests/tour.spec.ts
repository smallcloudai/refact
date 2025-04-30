import { test, expect } from "../fixtures";
import { LoginPage } from "../fixtures/LoginPage";
import { TourPage } from "../fixtures/TourPage";

test.use({ storageState: { cookies: [], origins: [] } });
test("Tour", async ({ page, baseURL, fakeIde, auth }) => {
  await fakeIde.clearMessages();
  await auth.doLogin(baseURL, false, false);

  const tourPage = new TourPage(page);

  await tourPage.step1();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.1 });
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step2();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot();

  await tourPage.step3();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot({
    maxDiffPixelRatio: 0.01,
  });

  await tourPage.step4();
  // await expect(page.getByTestId("tour-box")).toHaveScreenshot({
  //   maxDiffPixelRatio: 0.01,
  // });

  await tourPage.step5();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot({
    maxDiffPixelRatio: 0.01,
  });

  await tourPage.step6();
  await expect(page.getByTestId("tour-box")).toMatchAriaSnapshot();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot({
    maxDiffPixelRatio: 0.1,
  });

  await tourPage.step7();
  await expect(page.getByTestId("tour-box")).toHaveScreenshot({
    maxDiffPixelRatio: 0.01,
  });

  await tourPage.step8();
  await expect(page).toHaveScreenshot({ maxDiffPixelRatio: 0.01 });
});
