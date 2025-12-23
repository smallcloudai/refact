import type { Meta, StoryObj } from "@storybook/react";
import { LoginPage } from "./LoginPage";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../../components/Theme";
import {
  loginPollingGood,
  loginPollingWaiting,
  emailLogin,
  goodUser,
} from "../../__fixtures__/msw";

const App = () => {
  const store = setUpStore({
    config: {
      apiKey: null,
      host: "web",
      lspPort: 8001,
      addressURL: "Refact",
      themeProps: { appearance: "dark", accentColor: "gray" },
    },
    tour: {
      type: "finished",
    },
  });
  return (
    <Provider store={store}>
      <Theme>
        <LoginPage />
      </Theme>
    </Provider>
  );
};

const meta: Meta<typeof App> = {
  title: "Login",
  component: App,
} satisfies Meta<typeof LoginPage>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {},
  parameters: {
    msw: [
      loginPollingWaiting,
      loginPollingWaiting,
      loginPollingGood,
      emailLogin,
      goodUser,
    ],
  },
};
