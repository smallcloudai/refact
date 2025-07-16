import type { Meta, StoryObj } from "@storybook/react";
import { UserSurvey } from "./UserSurvey";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../../components/Theme";
import { http, HttpResponse, type HttpHandler } from "msw";
import { QUESTIONS_STUB } from "../../__fixtures__";
import { BasicStuff } from "../../__fixtures__/msw";

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

// TODO: needs graphql mocks
const meta = {
  title: "User Survey",
  component: Component,
  parameters: {
    msw: {
      handlers: [
        http.get("https://www.smallcloud.ai/v1/questionnaire", () => {
          return HttpResponse.json(QUESTIONS_STUB);
        }),
        BasicStuff,
      ],
    },
  },
} satisfies Meta<
  typeof Component & { parameters: { msw: { handlers: HttpHandler[] } } }
>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {};
