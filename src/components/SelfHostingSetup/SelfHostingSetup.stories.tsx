import type { Meta, StoryObj } from "@storybook/react";
import { SelfHostingSetup } from ".";
import { Flex } from "@radix-ui/themes";

const meta: Meta<typeof SelfHostingSetup> = {
  title: "Self hosting setup",
  component: SelfHostingSetup,
  args: {
    goBack: () => {
      // eslint-disable-next-line no-console
      console.log("goBack called");
    },
    next: (endpointAddress) => {
      // eslint-disable-next-line no-console
      console.log("next called with " + endpointAddress);
    },
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
