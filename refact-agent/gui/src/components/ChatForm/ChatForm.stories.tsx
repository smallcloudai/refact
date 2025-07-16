import React from "react";
import type { Meta, StoryObj } from "@storybook/react";

import { ChatForm } from "./ChatForm";
import { useDebounceCallback } from "usehooks-ts";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";
import { TourProvider } from "../../features/Tour";

// const _testCommands = [
//   "@workspace",
//   "@help",
//   "@list",
//   "@web",
//   "@database",
//   "@?",
//   "@longlonglonglonglonglonglonglonglonglong",
//   "@refactor",
//   "@test",
//   "@Apple",
//   "@Banana",
//   "@Carrot",
//   "@Dill",
//   "@Elderberries",
//   "@Figs",
//   "@Grapes",
//   "@Honeydew",
//   "@Iced melon",
//   "@Jackfruit",
//   "@Kale",
//   "@Lettuce",
//   "@Mango",
//   "@Nectarines",
//   "@Oranges",
//   "@Pineapple",
//   "@Quince",
//   "@Raspberries",
//   "@Strawberries",
//   "@Turnips",
//   "@Ugli fruit",
//   "@Vanilla beans",
//   "@Watermelon",
//   "@Xigua",
//   "@Yuzu",
//   "@Zucchini",
// ];

// const noop = () => ({});

const Template: React.FC<{ children: JSX.Element }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <TourProvider>
        <Theme>{children}</Theme>
      </TourProvider>
    </Provider>
  );
};

const meta: Meta<typeof ChatForm> = {
  title: "Chat Form",
  component: ChatForm,
  args: {
    onSubmit: (str) => {
      // eslint-disable-next-line no-console
      console.log("submit called with " + str);
    },
    onClose: () => {
      // eslint-disable-next-line no-console
      console.log("onclose called");
    },
  },
  decorators: [
    (Children) => {
      const requestCommandsCompletion = useDebounceCallback(() => ({}), 0);
      // TODO: use redux store
      // return (
      //   <ConfigProvider
      //     config={{ host: "vscode", features: { vecdb: true, ast: true } }}
      //   >
      //     <Children requestCommandsCompletion={requestCommandsCompletion} />
      //   </ConfigProvider>
      // );
      return (
        <Template>
          <Children requestCommandsCompletion={requestCommandsCompletion} />
        </Template>
      );
    },
  ],
} satisfies Meta<typeof ChatForm>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {},
};
