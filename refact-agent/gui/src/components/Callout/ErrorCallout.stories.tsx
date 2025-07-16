import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ErrorCallout } from ".";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";

const meta = {
  title: "Error Callout",
  component: ErrorCallout,
} satisfies Meta<typeof ErrorCallout>;

export default meta;

const Template: React.FC<{ children: JSX.Element }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>{children}</Theme>
    </Provider>
  );
};

export const Default: StoryObj<typeof ErrorCallout> = {
  args: {
    children: "some bad happened",
  },
  decorators: [
    (Story) => (
      <Template>
        <Story />
      </Template>
    ),
  ],
};
