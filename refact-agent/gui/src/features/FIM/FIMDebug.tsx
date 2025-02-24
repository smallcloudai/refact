import React from "react";
import { FIMDebug as FIMDebugView } from "../../components/FIMDebug";
import { useEventBusForFIMDebug } from "../../hooks";
import { Callout } from "../../components/Callout";
import { Spinner, Flex, Button } from "@radix-ui/themes";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { type Config } from "../Config/configSlice";
import { PageWrapper } from "../../components/PageWrapper";

export type FIMDebugProps = {
  host: Config["host"];
  tabbed: Config["tabbed"];
};

export const FIMDebug: React.FC<FIMDebugProps> = ({ host, tabbed }) => {
  const { state, clearErrorMessage, backFromFim } = useEventBusForFIMDebug();
  return (
    <PageWrapper host={host}>
      {!tabbed && (
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
