import type { Meta, StoryObj } from "@storybook/react";
import { Provider } from "react-redux";

import { setUpStore } from "../../../app/store";
import { Theme } from "../../Theme";
import { AbortControllerProvider } from "../../../contexts/AbortControllers";

import { UsageCounter } from ".";

const MockedStore: React.FC = () => {
  const store = setUpStore({
    config: {
      themeProps: {
        appearance: "dark",
      },
      host: "web",
      lspPort: 8001,
    },
  });

  return (
    <Provider store={store}>
      <AbortControllerProvider>
        <Theme accentColor="gray">
          <UsageCounter />
        </Theme>
      </AbortControllerProvider>
    </Provider>
  );
};

const meta: Meta<typeof MockedStore> = {
  title: "UsageCounter",
  component: MockedStore,
};

export default meta;

export const Default: StoryObj<typeof UsageCounter> = {};
