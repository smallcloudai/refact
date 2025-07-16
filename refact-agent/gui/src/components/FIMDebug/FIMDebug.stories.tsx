import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { FIMDebug } from ".";
import { STUB } from "../../__fixtures__/fim";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
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
  title: "FIM Debug Page",
  component: FIMDebug,
  args: {
    data: STUB,
  },
  decorators: [
    (Story) => (
      <Template>
        <Story />
      </Template>
    ),
  ],
} satisfies Meta<typeof FIMDebug>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
