import React from "react";

import type { Meta, StoryObj } from "@storybook/react";

import { ScrollArea } from "./ScrollArea";
import { Flex, Text } from "@radix-ui/themes";

const Content: React.ReactNode = (
  <Flex p="2" pr="8" direction="column" gap="4">
    <Text size="2" trim="both">
      Three fundamental aspects of typography are legibility, readability, and
      aesthetics. Although in a non-technical sense &quot;legible&quot; and
      &quot;readable&quot; are often used synonymously, typographically they are
      separate but related concepts.
    </Text>

    <Text size="2" trim="both">
      Legibility describes how easily individual characters can be distinguished
      from one another. It is described by Walter Tracy as &quot;the quality of
      being decipherable and recognizable&quot;. For instance, if a
      &quot;b&quot; and an &quot;h&quot;, or a &quot;3&quot; and an
      &quot;8&quot;, are difficult to distinguish at small sizes, this is a
      problem of legibility.
    </Text>
  </Flex>
);

const meta = {
  title: "Scroll Area",
  component: ScrollArea,
  args: {
    scrollbars: "vertical",
    style: { height: "150" },
  },
} satisfies Meta<typeof ScrollArea>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    children: Content,
  },
};
