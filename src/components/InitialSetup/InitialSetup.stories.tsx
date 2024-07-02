import type { Meta, StoryObj } from "@storybook/react";
import { InitialSetup } from ".";
import { Flex } from "@radix-ui/themes";

const meta: Meta<typeof InitialSetup> = {
  title: "InitialSetup",
  component: InitialSetup,
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
