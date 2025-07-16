import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Checkbox } from ".";
import { Flex } from "@radix-ui/themes";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";

const Template: React.FC<{ children: JSX.Element }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>
        <Flex p="4">{children}</Flex>
      </Theme>
    </Provider>
  );
};

const meta: Meta<typeof Checkbox> = {
  title: "Checkbox",
  component: Checkbox,
  decorators: [
    (Children) => (
      <Template>
        <Children />
      </Template>
    ),
  ],
} satisfies Meta<typeof Checkbox>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    name: "checkbox",
    children: "label text",
    title: "title text for help",
  },
};
