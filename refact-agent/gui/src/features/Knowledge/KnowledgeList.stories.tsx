import React from "react";
import type { Meta, StoryObj } from "@storybook/react";
import { KnowledgeList } from "./KnowledgeList";
import { Provider } from "react-redux";
import { Theme } from "../../components/Theme";
import { TourProvider } from "../Tour";
import { AbortControllerProvider } from "../../contexts/AbortControllers";
import { setUpStore } from "../../app/store";
import { knowLedgeLoading, KnowledgeWithStatus } from "../../__fixtures__/msw";
import { Container } from "@radix-ui/themes";

const Template: React.FC = () => {
  const store = setUpStore();
  return (
    <Provider store={store}>
      <Theme>
        <TourProvider>
          <AbortControllerProvider>
            <Container py="8" px="4">
              <KnowledgeList />
            </Container>
          </AbortControllerProvider>
        </TourProvider>
      </Theme>
    </Provider>
  );
};

const meta: Meta<typeof KnowledgeList> = {
  title: "KnowledgeList",
  component: Template,
};

export default meta;

type Story = StoryObj<typeof KnowledgeList>;

export const Primary: Story = {
  parameters: {
    msw: {
      handlers: [knowLedgeLoading],
    },
  },
};

export const LoadingVecDd: Story = {
  parameters: {
    msw: {
      handlers: [KnowledgeWithStatus],
    },
  },
};
