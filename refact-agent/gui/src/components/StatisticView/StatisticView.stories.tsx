import type { Meta, StoryObj } from "@storybook/react";
import { StatisticView } from "./StatisticView";
import { json as stub } from "../../__fixtures__/table";

const meta = {
  title: "StatisticView",
  component: StatisticView,
  args: {
    statisticData: stub,
    isLoading: false,
    error: "",
  },
} satisfies Meta<typeof StatisticView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
