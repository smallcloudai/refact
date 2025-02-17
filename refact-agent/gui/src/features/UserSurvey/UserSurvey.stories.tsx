import type { Meta, StoryObj } from "@storybook/react";
import { UserSurvey } from "./UserSurvey";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../../components/Theme";
import { http, HttpResponse, type HttpHandler } from "msw";
import { QUESTIONS_STUB } from "../../__fixtures__";

const Component = () => {
  const store = setUpStore({
    config: {
      apiKey: "test-key",
      host: "web",
      lspPort: 8001,
      addressURL: "Refact",
      themeProps: { appearance: "dark" },
    },
    userSurvey: {
      lastAsked: 0,
    },
  });
  return (
    <Provider store={store}>
      <Theme>
        <UserSurvey />
      </Theme>
    </Provider>
  );
};

const meta = {
  title: "User Survey",
  component: Component,
  parameters: {
    msw: {
      handlers: [
        http.get("http://127.0.0.1:8001/v1/ping", () => {
          return HttpResponse.text("pong");
        }),
        http.get("https://www.smallcloud.ai/v1/login", () => {
          return HttpResponse.json({
            retcode: "OK",
            account: "party@refact.ai",
            inference_url: "https://www.smallcloud.ai/v1",
            inference: "PRO",
            metering_balance: -100000,
            questionnaire: false,
          });
        }),
        http.get("https://www.smallcloud.ai/v1/questionnaire", () => {
          return HttpResponse.json(QUESTIONS_STUB);
        }),
      ],
    },
  },
} satisfies Meta<
  typeof Component & { parameters: { msw: { handlers: HttpHandler[] } } }
>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
