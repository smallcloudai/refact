import type { Meta, StoryObj } from "@storybook/react";
import { Callout } from ".";

const meta = {
  title: "Callout",
  component: Callout,
} satisfies Meta<typeof Callout>;

export default meta;

export const Default: StoryObj<typeof Callout> = {
  args: {
    children: "This is a callout",
  },
};
