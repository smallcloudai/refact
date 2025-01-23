import { Meta, StoryObj } from "@storybook/react";

import { ChatLinks } from "./ChatLinks";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";
import { Container } from "@radix-ui/themes";
import { http, HttpResponse, type HttpHandler } from "msw";
import { CHAT_LINKS_URL } from "../../services/refact/consts";
import {
  STUB_LINKS_FOR_CHAT_RESPONSE,
  CHAT_CONFIG_THREAD,
} from "../../__fixtures__";

const Template = () => {
  const store = setUpStore({
    chat: CHAT_CONFIG_THREAD,
  });
  return (
    <Provider store={store}>
      <Theme>
        <Container p="4">
          <ChatLinks />
        </Container>
      </Theme>
    </Provider>
  );
};

const meta = {
  title: "Components/ChatLinks",
  component: Template,
  argTypes: {
    //...
  },
  parameters: {
    msw: {
      handlers: [
        http.post(CHAT_LINKS_URL, () => {
          return HttpResponse.json(STUB_LINKS_FOR_CHAT_RESPONSE);
        }),
      ],
    },
  },
} satisfies Meta<
  typeof Template & { parameters: { msw: { handlers: HttpHandler[] } } }
>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
