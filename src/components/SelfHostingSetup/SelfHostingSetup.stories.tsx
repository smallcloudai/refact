import type { Meta, StoryObj } from "@storybook/react";
import { SelfHostingSetup } from ".";
import { Flex } from "@radix-ui/themes";
import { fn } from "@storybook/test";

const meta: Meta<typeof SelfHostingSetup> = {
  title: "Self hosting setup",
  component: SelfHostingSetup,
  args: {
    goBack: fn(),
    next: fn(),
  },
  decorators: [
    (Children) => (
      <Flex p="4">
        <Children />
      </Flex>
    ),
  ],
} satisfies Meta<typeof SelfHostingSetup>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {},
};
