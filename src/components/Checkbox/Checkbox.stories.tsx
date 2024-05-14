import type { Meta, StoryObj } from "@storybook/react";
import { Checkbox } from ".";
import { Flex } from "@radix-ui/themes";

const meta: Meta<typeof Checkbox> = {
  title: "Checkbox",
  component: Checkbox,
  decorators: [
    (Children) => (
      <Flex p="4">
        <Children />
      </Flex>
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
