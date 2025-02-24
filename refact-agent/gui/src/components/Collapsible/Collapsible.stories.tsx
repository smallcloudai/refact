import type { Meta, StoryObj } from "@storybook/react";
import { Collapsible } from ".";
import { Text } from "../Text";
import { Flex } from "@radix-ui/themes";

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
};
