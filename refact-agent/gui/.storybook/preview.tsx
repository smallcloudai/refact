import type { Preview } from "@storybook/react";
import "@radix-ui/themes/styles.css";
import "../src/lib/render/web.css";

import { initialize, mswLoader } from "msw-storybook-addon";

initialize();

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
  loaders: [mswLoader],
};

export default preview;
