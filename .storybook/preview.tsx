import React from "react";
import type { Preview } from "@storybook/react";
import "@radix-ui/themes/styles.css";
import { Theme } from "../src/components/Theme";

const preview: Preview = {
  parameters: {
    actions: { argTypesRegex: "^on[A-Z].*" },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
    layout: "fullscreen",
  },
  decorators: [
    (Page) => (
      <Theme accentColor="gray">
        <Page />
      </Theme>
    ),
  ],
};

export default preview;
