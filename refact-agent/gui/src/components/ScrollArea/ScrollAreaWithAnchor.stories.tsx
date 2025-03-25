import React from "react";

import type { Meta, StoryObj } from "@storybook/react";

import { ScrollAreaWithAnchor } from ".";
import { Text, Container, Theme, Card } from "@radix-ui/themes";

const meta: Meta<typeof ScrollAreaWithAnchor.ScrollArea> = {
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
};

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

export const InTheMiddle: Story = {
  args: {
    children: [
      <Text as="p" key="ipsum-1" wrap="wrap" mb="8">
        Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nam lacinia
        pulvinar tortor nec facilisis. Pellentesque dapibus efficitur laoreet.
        Nam risus ante, dapibus a molestie consequat, ultrices ac magna. Fusce
        dui lectus, congue vel laoreet ac, dictum vitae odio. Donec aliquet.
        Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nam lacinia
        pulvinar tortor nec facilisis. Pellentesque dapibus efficitur laoreet.
        Nam risus ante, dapibus a molestie consequat, ultrices ac magna. Fusce
        dui lectus, congue vel laoreet ac, dictum vitae odio. Donec aliquet.
        Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nam lacinia
        pulvinar tortor nec facilisis. Pellentesque dapibus efficitur laoreet.
        Sed ut perspiciatis unde omnis iste natus error sit voluptatem
        accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab
        illo inventore veritatis et quasi architecto beatae vitae dicta sunt
        explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut
        odit aut fugit, sed quia consequuntur magni dolores eos qui ratione
        voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum
        quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam
        eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat
        voluptatem. Ut enim ad minima veniam, quis nostrum exercitationem ullam
        corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur.
      </Text>,
      <ScrollAreaWithAnchor.ScrollAnchor
        key="anchor"
        behavior="smooth"
        block="start"
      />,
      <Text as="div" key="end">
        👋
      </Text>,
      <Text as="p" key="ipsum-2" wrap="wrap" mt="8">
        Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nam lacinia
        pulvinar tortor nec facilisis. Pellentesque dapibus efficitur laoreet.
        Nam risus ante, dapibus a molestie consequat, ultrices ac magna. Fusce
        dui lectus, congue vel laoreet ac, dictum vitae odio. Donec aliquet.
        Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nam lacinia
        pulvinar tortor nec facilisis. Pellentesque dapibus efficitur laoreet.
        Nam risus ante, dapibus a molestie consequat, ultrices ac magna. Fusce
        dui lectus, congue vel laoreet ac, dictum vitae odio. Donec aliquet.
        Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nam lacinia
        pulvinar tortor nec facilisis. Pellentesque dapibus efficitur laoreet.
        Sed ut perspiciatis unde omnis iste natus error sit voluptatem
        accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae ab
        illo inventore veritatis et quasi architecto beatae vitae dicta sunt
        explicabo. Nemo enim ipsam voluptatem quia voluptas sit aspernatur aut
        odit aut fugit, sed quia consequuntur magni dolores eos qui ratione
        voluptatem sequi nesciunt. Neque porro quisquam est, qui dolorem ipsum
        quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam
        eius modi tempora incidunt ut labore et dolore magnam aliquam quaerat
        voluptatem. Ut enim ad minima veniam, quis nostrum exercitationem ullam
        corporis suscipit laboriosam, nisi ut aliquid ex ea commodi consequatur.
      </Text>,
    ],
  },
};
