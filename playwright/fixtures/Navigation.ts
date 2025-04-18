import { Page } from "@playwright/test";

export class Navigation {
  constructor(public readonly page: Page) {}

  get homeButton() {
    return this.page.getByText("HomeHome");
  }

  homeUrl() {
    this.page.goto("/");
  }

  get menuButton() {
    return this.page.getByRole("button", { name: "menu" });
  }
}
