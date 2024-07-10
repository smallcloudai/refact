import type { Meta, StoryObj } from "@storybook/react";
import { InitialSetup } from ".";
import { Flex } from "@radix-ui/themes";
import { fn } from "@storybook/test";

const meta: Meta<typeof InitialSetup> = {
  title: "Initial setup",
  component: InitialSetup,
  args: {
    onPressNext: fn(),
  },
  decorators: [
    (Children) => (
      <Flex p="4">
        <Children />
      </Flex>
    ),
  ],
} satisfies Meta<typeof InitialSetup>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {},
};
