import { test as baseTest } from "@playwright/test";
import { LoginPage } from "./LoginPage";
import { TourPage } from "./TourPage";
import { Navigation } from "./Navigation";
import { FakeIde } from "./FakeIde";

export * from "@playwright/test";
export const test = baseTest.extend<
  {
    navigation: Navigation;
    fakeIde: FakeIde;
    auth: LoginPage;
  },
  { workerStorageState: string }
>({
  // // Use the same storage state for all tests in this worker.
  // storageState: ({ workerStorageState }, use) => use(workerStorageState),

  // // Authenticate once per worker with a worker-scoped fixture.
  // workerStorageState: [
  //   async ({ browser }, use) => {
  //     // Use parallelIndex as a unique identifier for each worker.
  //     const id = test.info().parallelIndex;
  //     const fileName = path.resolve(
  //       test.info().project.outputDir,
  //       ".auth",
  //       `${id}.json`
  //     );

  //     if (fs.existsSync(fileName)) {
  //       // Reuse existing authentication state if any.
  //       await use(fileName);
  //       return;
  //     }

  //     // Important: make sure we authenticate in a clean environment by unsetting storage state.
  //     const page = await browser.newPage({
  //       storageState: undefined,
  //     });

  //     const fakeIde = await FakeIde.initialize(page);
  //     const loginPage = new LoginPage(page);
  //     await loginPage.doLogin(undefined, false, false);
  //     const tourPage = new TourPage(page);
  //     await tourPage.doTour();
  //     await page.context().storageState({ path: fileName });
  //     await page.close();
  //     await use(fileName);
  //   },
  //   { scope: "worker" },
  // ],

  navigation: async ({ page }, use) => {
    const navigation = new Navigation(page);
    await use(navigation);
  },

  fakeIde: async ({ page }, use) => {
    const fakeIde = await FakeIde.initialize(page);
    await use(fakeIde);
  },

  auth: async ({ page }, use) => {
    const auth = new LoginPage(page);
    await use(auth);
  },
});

test.use({
  storageState: {
    cookies: [],
    origins: [
      {
        origin: "http://localhost:3000/",
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
