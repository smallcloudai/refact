import type { Meta, StoryObj } from "@storybook/react";
import { EnterpriseSetup } from ".";
import { Flex } from "@radix-ui/themes";
import { fn } from "@storybook/test";

const meta: Meta<typeof EnterpriseSetup> = {
  title: "Enterprise setup",
  component: EnterpriseSetup,
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
} satisfies Meta<typeof EnterpriseSetup>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {},
};
