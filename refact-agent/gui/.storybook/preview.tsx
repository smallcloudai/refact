import type { Preview } from "@storybook/react";
import "@radix-ui/themes/styles.css";
import "../src/lib/render/web.css";

import { initialize, mswLoader } from "msw-storybook-addon";

initialize({
  onUnhandledRequest: (request, print) => {
    if (request.url.startsWith("http://localhost:6006/src/")) {
      return;
    }
    print.warning();
  },
});

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
