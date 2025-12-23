import type { Meta, StoryObj } from "@storybook/react";
import { Select } from ".";
import { Theme, Container } from "@radix-ui/themes";

const meta: Meta<typeof Select> = {
  title: "Select",
  component: Select,
  decorators: [
    (Story) => (
      <Theme>
        <Container>
          <Story />
        </Container>
      </Theme>
    ),
  ],
};

export default meta;

const long = "long".repeat(30);

export const Default: StoryObj<typeof Select> = {
  args: {
    options: ["apple", "banana", "orange", long],
    onChange: () => ({}),
    defaultValue: "apple",
  },
};

export const OptionObject: StoryObj<typeof Select> = {
  args: {
    options: [
      { value: "apple" },
      { value: "banana", disabled: true },
      { value: "orange" },
      { value: long },
    ],
    onChange: () => ({}),
    defaultValue: "apple",
  },
};
