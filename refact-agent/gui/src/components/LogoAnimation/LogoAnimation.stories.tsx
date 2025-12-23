import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { LogoAnimation } from "./LogoAnimation";
import { sizeValues, defaultSize } from "./types";
import { Theme } from "../Theme";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Card, Container } from "@radix-ui/themes";

const App: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>
        <Container p="8">
          <Card>{children}</Card>
        </Container>
      </Theme>
    </Provider>
  );
};
const meta: Meta<typeof LogoAnimation> = {
  title: "Logo Animation",
  component: LogoAnimation,
  argTypes: {
    size: {
      defaultValue: defaultSize,
      control: "select",
      options: sizeValues,
    },
  },
  decorators: [
    (Story) => (
      <App>
        <Story />
      </App>
    ),
  ],
};

export default meta;

type Story = StoryObj<typeof LogoAnimation>;

export const Primary: Story = {};
