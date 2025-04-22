import type { Meta, StoryObj } from "@storybook/react";
import { Theme, Card, ThemeProps, ThemePanel } from "@radix-ui/themes";
import { AnimatedText } from "./AnimatedText";

const meta: Meta<typeof AnimatedText> = {
  title: "Components/Text/Animated",
  component: AnimatedText,
  decorators: [
    (Elem) => (
      <Theme appearance="inherit" accentColor="gray">
        <ThemePanel />
        <Card>
          <Elem />
        </Card>
      </Theme>
    ),
  ],
};

export default meta;

type Story = StoryObj<typeof AnimatedText>;

export const Primary: Story = {
  args: {
    animate: true,

    children: "Hello World",
  },
};
