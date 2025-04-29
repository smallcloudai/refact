import { expect, type Page } from "@playwright/test";

export class TourPage {
  constructor(public readonly page: Page) {}

  async step1() {
    await expect(this.page.getByText("Welcome to Refact.ai!")).toBeVisible();
  }
  async step2() {
    await this.page.getByRole("button", { name: "Get Started" }).click();
    await expect(
      this.page.getByText("Agent can accomplish tasks")
    ).toBeVisible();
  }

  async step3() {
    await this.page.getByText("next").click();
    await expect(
      this.page.getByText("Integrations", { exact: true })
    ).toBeVisible();
  }

  async step4() {
    await this.page.getByText("next").click();
    await expect(this.page.getByText("Chat modes / models")).toBeVisible();
  }

  async step5() {
    await this.page.getByText("next").click();
    await expect(
      this.page.getByText("Difference in Quick / Explore")
    ).toBeVisible();
  }

  async step6() {
    await this.page.getByText("next").click();
    await expect(this.page.getByText("Code completion")).toBeVisible();
  }

  async step7() {
    await this.page.getByText("next").click();
    await expect(
      this.page.getByText("Your Refact product tour is finished!")
    ).toBeVisible();
  }

  async step8() {
    await this.page.getByRole("button", { name: "Ready to use" }).click();
  }

  async doTour() {
    await this.step1();
    await this.step2();
    await this.step3();
    await this.step4();
    await this.step5();
    await this.step6();
    await this.step7();
    await this.step8();
  }
}
