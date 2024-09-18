import type { Meta, StoryObj } from "@storybook/react";
import { CloudLogin } from ".";
import { Flex } from "@radix-ui/themes";
import { fn } from "@storybook/test";

const meta: Meta<typeof CloudLogin> = {
  title: "Cloud Login",
  component: CloudLogin,
  args: {
    goBack: fn(),
  },
  decorators: [
    (Children) => {
      return (
        <Flex p="4">
          <Children />
        </Flex>
      );
    },
  ],
} satisfies Meta<typeof CloudLogin>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {},
};
