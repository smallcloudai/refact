import type { Meta, StoryObj } from "@storybook/react";
import { Sidebar } from "./Sidebar";

const meta = {
  title: "Sidebar",
  component: Sidebar,
  args: {
    history: []
  },
} satisfies Meta<typeof Sidebar>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
