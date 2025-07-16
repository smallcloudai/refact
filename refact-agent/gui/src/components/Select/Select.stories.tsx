import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Select } from ".";
import { Container } from "@radix-ui/themes";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";

const Template: React.FC<{ children: JSX.Element }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>
        <Container p="8">{children}</Container>
      </Theme>
    </Provider>
  );
};

const meta: Meta<typeof Select> = {
  title: "Select",
  component: Select,
  decorators: [
    (Story) => (
      <Template>
        <Story />
      </Template>
    ),
  ],
};

export default meta;

const long = "long".repeat(30);

export const Default: StoryObj<typeof Select> = {
  args: {
    options: ["apple", "banana", "orange", long],
    onChange: () => ({}),
    defaultValue: "apple",
  },
};

export const OptionObject: StoryObj<typeof Select> = {
  args: {
    options: [
      { value: "apple" },
      { value: "banana", disabled: true },
      { value: "orange" },
      { value: long },
    ],
    onChange: () => ({}),
    defaultValue: "apple",
  },
};
