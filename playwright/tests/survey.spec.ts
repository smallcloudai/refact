import { test, expect } from "../fixtures";
import { LoginPage } from "../fixtures/LoginPage";
import { TourPage } from "../fixtures/TourPage";

test.use({
  storageState: {
    cookies: [],
    origins: [
      {
        origin: "http://localhost:5173/",
        localStorage: [
          {
            name: "persist:root",
            value: JSON.stringify({
              tour: JSON.stringify({ type: "finished", step: 1 }),
            }),
          },
        ],
      },
    ],
  },
});

test("User Survey", async ({ page, baseURL }) => {
  const loginPage = new LoginPage(page);
  await loginPage.doLogin(baseURL, true, false);
  // await page.goto(baseURL ?? "/");

  await expect(page.getByRole("dialog")).toHaveScreenshot();

  await page.locator("label").filter({ hasText: "Other" }).click();

  await page.getByRole("textbox", { name: "Other..." }).fill("testing");

  await page
    .context()
    .route(
      "http://www.smallcloud.ai/save-questionnaire",
      async (route, request) => {
        await expect(request.method).toBe("POST");
        const body = request.postDataJSON();
        await expect(body).toEqual({});
        route.fulfill({ status: 200 });
      }
    );

  await page.getByRole("button", { name: "Submit" }).click();

  await page.waitForLoadState("networkidle");

  await expect(
    page.getByRole("dialog", { name: "Thank You" })
  ).toHaveScreenshot();
});
