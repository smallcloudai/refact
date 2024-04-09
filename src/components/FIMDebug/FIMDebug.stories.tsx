import type { Meta, StoryObj } from "@storybook/react";
import { FIMDebug } from ".";
import { STUB } from "../../__fixtures__/fim";

const meta = {
  title: "FIM Debug Page",
  component: FIMDebug,
  args: {
    data: STUB,
  },
} satisfies Meta<typeof FIMDebug>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
