import React from "react";
import type { Meta } from "@storybook/react";
import * as Accordion from "./Accordion";
import { Theme } from "../Theme";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Container } from "@radix-ui/themes";

const App: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>
        <Container p="8">{children}</Container>
      </Theme>
    </Provider>
  );
};
const meta: Meta<typeof Accordion> = {
  title: "Accordion",
  decorators: [
    (Story) => (
      <App>
        <Story />
      </App>
    ),
  ],
};

export default meta;

export const Primary = () => {
  return (
    <Accordion.Root type="single" defaultValue="item-1" collapsible>
      <Accordion.Item value="item-1">
        <Accordion.Trigger>Item one</Accordion.Trigger>
        <Accordion.Content>Content for item one</Accordion.Content>
      </Accordion.Item>

      <Accordion.Item value="item-2">
        <Accordion.Trigger>Item 2</Accordion.Trigger>
        <Accordion.Content>Content for item two</Accordion.Content>
      </Accordion.Item>

      <Accordion.Item value="item-3">
        <Accordion.Trigger>Item 3</Accordion.Trigger>
        <Accordion.Content>
          <div>Content for item 3</div>
        </Accordion.Content>
      </Accordion.Item>
    </Accordion.Root>
  );
};
