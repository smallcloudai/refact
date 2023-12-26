import type { Meta, StoryObj } from "@storybook/react";

import { Spinner } from "./Spinner";

const meta = {
  title: "Spinner",
  component: Spinner,
} satisfies Meta<typeof Spinner>;

export default meta;

export const Primary: StoryObj<typeof Spinner> = {};
