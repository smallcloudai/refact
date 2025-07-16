import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { StatisticView } from "./StatisticView";
import { json as stub } from "../../__fixtures__/table";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../Theme";

const Template: React.FC<{ children: JSX.Element }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>{children}</Theme>
    </Provider>
  );
};

const meta = {
  title: "StatisticView",
  component: StatisticView,
  args: {
    statisticData: stub,
    isLoading: false,
    error: "",
  },
  decorators: [
    (Story) => (
      <Template>
        <Story />
      </Template>
    ),
  ],
} satisfies Meta<typeof StatisticView>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
