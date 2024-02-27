import type { Meta, StoryObj } from "@storybook/react";
import { StatisticView } from "./StatisticView";
import { json as stub } from "../../__fixtures__/table";
import { CONTEXT_FILES } from "../../__fixtures__";

const meta = {
  title: "StatisticView",
  component: StatisticView,
  args: {
    statisticData: stub,
    isLoading: false,
    error: "",
    fimFiles: {
      error: "",
      fetching: false,
      files: CONTEXT_FILES,
    },
  },
} satisfies Meta<typeof StatisticView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
