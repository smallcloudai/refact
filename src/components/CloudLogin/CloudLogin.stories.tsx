import type { Meta, StoryObj } from "@storybook/react";
import { CloudLogin } from ".";
import { Flex } from "@radix-ui/themes";

const meta: Meta<typeof CloudLogin> = {
  title: "CloudLogin",
  component: CloudLogin,
  args: {},
  decorators: [
    (Children) => (
      <Flex p="4">
        <Children />
      </Flex>
    ),
  ],
} satisfies Meta<typeof CloudLogin>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {},
};
