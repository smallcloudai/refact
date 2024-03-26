import type { Meta, StoryObj } from "@storybook/react";
import { Select } from ".";

const meta = {
  title: "Select",
  component: Select,
} satisfies Meta<typeof Select>;

export default meta;

const long = "long".repeat(30);

export const Default: StoryObj<typeof Select> = {
  args: {
    options: ["apple", "banana", "orange", long],
    onChange: () => ({}),
    defaultValue: "apple",
  },
};
