import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Spinner } from "./Spinner";
import { Box, Heading } from "@radix-ui/themes";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";

const App: React.FC = () => {
  const store = setUpStore({
    config: {
      apiKey: "test-key",
      host: "web",
      lspPort: 8001,
      addressURL: "Refact",
      themeProps: { appearance: "dark" },
    },
  });
  return (
    <Provider store={store}>
      <Theme>
        <Box>
          <Box>
            <Heading>Spinning</Heading>
            <Spinner spinning />
          </Box>
          <Box>
            <Heading>Not Spinning</Heading>
            <Spinner spinning={false} />
          </Box>
          <Box>
            <Heading>Spinning</Heading>
            <Spinner spinning />
          </Box>
          <Box>
            <Heading>Not Spinning</Heading>
            <Spinner spinning={false} />
          </Box>
          <Box>
            <Heading>Spinning</Heading>
            <Spinner spinning />
          </Box>
          <Box>
            <Heading>Not Spinning</Heading>
            <Spinner spinning={false} />
          </Box>
        </Box>
      </Theme>
    </Provider>
  );
};

const meta = {
  title: "Spinner",
  component: App,
} satisfies Meta<typeof Spinner>;

export default meta;

export const Primary: StoryObj<typeof Spinner> = {};
