import type { Meta, StoryObj } from "@storybook/react";
import { Select } from ".";

const meta = {
  title: "Select",
  component: Select,
} satisfies Meta<typeof Select>;

export default meta;

export const Default: StoryObj<typeof Select> = {
  args: {
    options: ["apple", "banana", "orange"].map((value) => ({
      value,
      label: value,
    })),
    onChange: () => ({}),
    defaultValue: "apple",
  },
};
