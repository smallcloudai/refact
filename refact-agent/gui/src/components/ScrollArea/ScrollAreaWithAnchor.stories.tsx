import React from "react";

import type { Meta, StoryObj } from "@storybook/react";

import { ScrollAreaWithAnchor } from ".";
import { Flex, Text, Container, Theme, Card } from "@radix-ui/themes";

const meta = {
  title: "Scroll Area Anchor",
  decorators: [
    (Story) => (
      <Theme>
        <Container p="8" maxHeight="100%">
          <Card>
            <Story />
          </Card>
        </Container>
      </Theme>
    ),
  ],
  parameters: {
    controls: { expanded: true },
  },
  component: ScrollAreaWithAnchor.ScrollArea,
  args: {
    scrollbars: "vertical",
    fullHeight: true,
    style: { height: "150px" },
  },
} satisfies Meta<typeof ScrollAreaWithAnchor.ScrollArea>;

export default meta;
type Story = StoryObj<typeof meta>;

const TopText: React.FC = () => {
  return Array.from({ length: 100 }).map((_, i) => {
    return (
      <Text key={i} as="p">
        {i + 1}
      </Text>
    );
  });
};

export const Primary: Story = {
  args: {
    children: [
      <TopText key="top-text" />,
      <ScrollAreaWithAnchor.ScrollAnchor
        key="anchor"
        behavior="smooth"
        block="start"
      />,
      <Text key="end">Scroll up</Text>,
    ],
  },
};

export const Short: Story = {
  args: {
    children: [
      <Text key="top">Should be offscreen</Text>,
      <ScrollAreaWithAnchor.ScrollAnchor
        key="anchor"
        behavior="smooth"
        block="start"
      />,
      <Text key="end">Scroll up</Text>,
    ],
  },
};
