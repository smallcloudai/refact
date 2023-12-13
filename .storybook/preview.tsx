import React from "react";
import type { Preview } from "@storybook/react";
import "@radix-ui/themes/styles.css";
import { Theme } from "@radix-ui/themes";

const preview: Preview = {
  parameters: {
    actions: { argTypesRegex: "^on[A-Z].*" },
    controls: {
      matchers: {
        color: /(background|color)$/i,
        date: /Date$/i,
      },
    },
  },
  decorators: [
    (Page) => (
      <Theme>
        <Page />
      </Theme>
    ),
  ],
};

export default preview;
