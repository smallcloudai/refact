import type { Meta, StoryObj } from "@storybook/react";
import { EnterpriseSetup } from ".";
import { Flex } from "@radix-ui/themes";

const meta: Meta<typeof EnterpriseSetup> = {
  title: "Enterprise setup",
  component: EnterpriseSetup,
  args: {
    goBack: () => {
      // eslint-disable-next-line no-console
      console.log("goBack called");
    },
    next: (endpointAddress, apiKey) => {
      // eslint-disable-next-line no-console
      console.log(
        "next called with " + JSON.stringify({ endpointAddress, apiKey }),
      );
    },
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
