import React from "react";
import { FIMDebug as FIMDebugView } from "../components/FIMDebug";
import { useEventBysForFIMDebug } from "../hooks";
import { Callout } from "../components/Callout";
import { Spinner, Flex, Button } from "@radix-ui/themes";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { useConfig } from "../contexts/config-context";
import { PageWrapper } from "../components/PageWrapper";

export const FIMDebug: React.FC = () => {
  const { host, tabbed } = useConfig();
  const { state, clearErrorMessage, backFromFim } = useEventBysForFIMDebug();

  return (
    <PageWrapper host={host}>
      {host === "vscode" && !tabbed && (
        <Flex gap="2" p="2" wrap="wrap">
          <Button size="1" variant="surface" onClick={backFromFim}>
            <ArrowLeftIcon width="16" height="16" />
            Back
          </Button>
        </Flex>
      )}
      {state.data ? (
        <FIMDebugView data={state.data} />
      ) : state.error ? (
        <Callout type="info" onClick={clearErrorMessage}>
          {state.error}
        </Callout>
      ) : state.fetching ? (
        <Spinner />
      ) : (
        <Callout type="info">
          No Fill in middle data available, try to make a completion
        </Callout>
      )}
    </PageWrapper>
  );
};
