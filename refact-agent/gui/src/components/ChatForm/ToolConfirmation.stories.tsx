import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ToolConfirmation } from "./ToolConfirmation";
import { Provider } from "react-redux";
import { setUpStore } from "../../app/store";
import { Theme } from "../Theme";
import { ToolConfirmationPauseReason } from "../../services/refact";
import { AbortControllerProvider } from "../../contexts/AbortControllers";
import {
  CONFIRMATIONAL_PAUSE_REASONS,
  CONFIRMATIONAL_PAUSE_REASONS_WITH_PATH,
  DENIAL_PAUSE_REASONS_WITH_PATH,
  MIXED_PAUSE_REASONS,
} from "../../__fixtures__/confirmation";

const MockedStore: React.FC<{
  pauseReasons: ToolConfirmationPauseReason[];
}> = ({ pauseReasons }) => {
  const store = setUpStore({
    confirmation: {
      pauseReasons,
      pause: true,
      status: {
        wasInteracted: false,
        confirmationStatus: false,
      },
    },
  });

  return (
    <Provider store={store}>
      <AbortControllerProvider>
        <Theme accentColor="gray">
          <ToolConfirmation pauseReasons={pauseReasons} />
        </Theme>
      </AbortControllerProvider>
    </Provider>
  );
};

const meta: Meta<typeof MockedStore> = {
  title: "ToolConfirmation",
  component: MockedStore,
  args: {
    pauseReasons: [],
  },
};

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    pauseReasons: CONFIRMATIONAL_PAUSE_REASONS_WITH_PATH,
  },
};

export const WithDenial: Story = {
  args: {
    pauseReasons: DENIAL_PAUSE_REASONS_WITH_PATH,
  },
};

export const Patch: Story = {
  args: {
    pauseReasons: CONFIRMATIONAL_PAUSE_REASONS,
  },
};

export const Mixed: Story = {
  args: {
    pauseReasons: MIXED_PAUSE_REASONS,
  },
};
