// TODO: handle the survey if it shows up.

import { Page } from "@playwright/test";

class Survey {
  constructor(public readonly page: Page) {}

  dismissSurvey() {}
}
