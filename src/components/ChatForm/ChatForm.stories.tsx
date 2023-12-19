import type { Meta, StoryObj } from "@storybook/react";

import { ChatForm } from "./ChatForm";

const meta = {
  title: "Chat Form",
  component: ChatForm,
  args: {
    onSubmit: (str) => {
      // eslint-disable-next-line no-console
      console.log("submit called with " + str);
    },
    onClose: () => {
      // eslint-disable-next-line no-console
      console.log("onclose called");
    },
  },
} satisfies Meta<typeof ChatForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
