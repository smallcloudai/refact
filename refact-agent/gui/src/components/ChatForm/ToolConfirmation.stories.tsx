import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { ToolConfirmation } from "./ToolConfirmation";

import { Theme } from "../Theme";

import {
  CONFIRMATIONAL_PAUSE_REASONS,
  CONFIRMATIONAL_PAUSE_REASONS_WITH_PATH,
  DENIAL_PAUSE_REASONS_WITH_PATH,
  MIXED_PAUSE_REASONS,
} from "../../__fixtures__/confirmation";
import { ToolConfirmationRequest } from "../../features/ThreadMessages/threadMessagesSlice";

const MockedStore: React.FC<{
  toolConfirmationRequests: ToolConfirmationRequest[];
}> = (props) => {
  return (
    <Theme accentColor="gray">
      <ToolConfirmation {...props} />
    </Theme>
  );
};

const meta: Meta<typeof MockedStore> = {
  title: "ToolConfirmation",
  component: MockedStore,
  args: {
    toolConfirmationRequests: [],
  },
};

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    toolConfirmationRequests: CONFIRMATIONAL_PAUSE_REASONS_WITH_PATH,
  },
};

export const WithDenial: Story = {
  args: {
    toolConfirmationRequests: DENIAL_PAUSE_REASONS_WITH_PATH,
  },
};

export const Patch: Story = {
  args: {
    toolConfirmationRequests: CONFIRMATIONAL_PAUSE_REASONS,
  },
};

export const Mixed: Story = {
  args: {
    toolConfirmationRequests: MIXED_PAUSE_REASONS,
  },
};
