import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { setUpStore } from "../../app/store";
import { Provider } from "react-redux";
import { Theme } from "../../components/Theme";
import { Checkpoints } from "./Checkpoints";
import { CheckpointsMeta } from "./checkpointsSlice";
import {
  STUB_PREVIEWED_CHECKPOINTS_STATE,
  STUB_RESTORED_CHECKPOINTS_STATE_WITH_NO_CHANGES,
} from "../../__fixtures__/checkpoints";

const Template: React.FC<{ initialState?: CheckpointsMeta }> = ({
  initialState,
}) => {
  const store = setUpStore({
    tour: {
      type: "finished",
    },
    config: {
      apiKey: "foo",
      addressURL: "Refact",
      host: "web",
      lspPort: 8001,
      themeProps: {
        appearance: "dark",
      },
    },
    checkpoints: initialState ?? STUB_PREVIEWED_CHECKPOINTS_STATE,
  });

  return (
    <Provider store={store}>
      <Theme>
        <Checkpoints />
      </Theme>
    </Provider>
  );
};

const meta = {
  title: "Features/Checkpoints",
  component: Template,
  parameters: {
    layout: "centered",
  },
} satisfies Meta<typeof Template>;

export default meta;
type Story = StoryObj<typeof Template>;

export const Default: Story = {};

export const WithNoChanges: Story = {
  args: {
    initialState: STUB_RESTORED_CHECKPOINTS_STATE_WITH_NO_CHANGES,
  },
};

export const DialogClosed: Story = {
  args: {
    initialState: STUB_RESTORED_CHECKPOINTS_STATE_WITH_NO_CHANGES,
  },
};
