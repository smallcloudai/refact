import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { Sidebar, SidebarProps } from "./Sidebar";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../Theme";

const App: React.FC<SidebarProps> = (props) => {
  return <Sidebar {...props} style={{ width: "260px", flexShrink: 0 }} />;
};

const Template: React.FC<{ children: JSX.Element }> = ({ children }) => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>{children}</Theme>
    </Provider>
  );
};

const meta = {
  title: "Sidebar",
  component: App,

  decorators: [
    (Story) => (
      <Template>
        <Story />
      </Template>
    ),
  ],
  args: {
    // history: [
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    //   ...HISTORY,
    // ],
    takingNotes: false,
    // currentChatId: "",
  },
} satisfies Meta<typeof App>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  // args: {
  //   account: {
  //     email: "user@example.org",
  //     tokens: 1800,
  //     plan: "Pro",
  //   },
  // },
};

export const Cloud: Story = {};
