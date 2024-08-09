import type { Meta, StoryObj } from "@storybook/react";
import { Sidebar, SidebarProps } from "./Sidebar";
// import { HISTORY } from "../../__fixtures__/history";

const App: React.FC<SidebarProps> = (props) => {
  return <Sidebar {...props} style={{ width: "260px", flexShrink: 0 }} />;
};

const meta = {
  title: "Sidebar",
  component: App,
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
    handleLogout: () => ({}),
  },
} satisfies Meta<typeof App>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  args: {
    account: {
      email: "user@example.org",
      tokens: 1800,
      plan: "Pro",
    },
  },
};

export const Cloud: Story = {};
