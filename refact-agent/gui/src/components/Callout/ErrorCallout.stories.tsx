import type { Meta, StoryObj } from "@storybook/react";
import { ErrorCallout } from ".";

const meta = {
  title: "Error Callout",
  component: ErrorCallout,
} satisfies Meta<typeof ErrorCallout>;

export default meta;

export const Default: StoryObj<typeof ErrorCallout> = {
  args: {
    children: "some bad happened",
  },
};
