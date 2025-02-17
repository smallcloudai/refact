import type { Meta, StoryObj } from "@storybook/react";
import { DocumentationSettings } from ".";
import { Flex } from "@radix-ui/themes";
import { fn } from "@storybook/test";

const meta: Meta<typeof DocumentationSettings> = {
  title: "Documentation settings",
  component: DocumentationSettings,
  args: {
    sources: [
      {
        url: "https://docs.rs/url/latest/url/index.html",
        pages: 20,
        maxDepth: 2,
        maxPages: 50,
      },
      {
        url: "https://en.cppreference.com/w/cpp/string",
        pages: 1,
        maxDepth: 2,
        maxPages: 50,
      },
    ],
    editDocumentation: fn(),
    addDocumentation: fn(),
    deleteDocumentation: fn(),
  },
  decorators: [
    (Children) => (
      <Flex p="4">
        <Children />
      </Flex>
    ),
  ],
} satisfies Meta<typeof DocumentationSettings>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {},
};
