import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Collapsible } from ".";
import { Text } from "../Text";
import { Flex } from "@radix-ui/themes";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
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

const meta = {
  title: "Collapsible",
  component: Collapsible,
} satisfies Meta<typeof Collapsible>;

export default meta;

export const Default: StoryObj<typeof Collapsible> = {
  args: {
    title: "Collapsible",
    children: (
      <Flex direction="column">
        <Text>Item 1</Text>
        <Text>Item 2</Text>
        <Text>Item 3</Text>
      </Flex>
    ),
  },
  decorators: [
    (Story) => (
      <Template>
        <Story />
      </Template>
    ),
  ],
};
