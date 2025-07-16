import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { TextArea } from "./TextArea";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";
import { Card, Container } from "@radix-ui/themes";

const Template: React.FC<{ children: JSX.Element }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>
        <Container>
          <Card>{children}</Card>
        </Container>
      </Theme>
    </Provider>
  );
};

const meta = {
  title: "TextArea",
  component: TextArea,
  decorators: [
    (Story) => (
      <Template>
        <Story />
      </Template>
    ),
  ],
} satisfies Meta<typeof TextArea>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: { onChange: () => ({}) },
};
