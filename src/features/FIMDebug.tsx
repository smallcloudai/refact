import React from "react";
import { FIMDebug as FIMDebugView } from "../components/FIMDebug";
import { useEventBysForFIMDebug } from "../hooks";
import { Callout } from "../components/Callout";
import { Spinner } from "@radix-ui/themes";

export const FIMDebug: React.FC = () => {
  const { state, clearErrorMessage } = useEventBysForFIMDebug();
  if (state.data) return <FIMDebugView data={state.data} />;
  if (state.error)
    return (
      <Callout type="info" onClick={clearErrorMessage}>
        {state.error}
      </Callout>
    );
  if (state.fetching) return <Spinner />;
  // TODO: add some default message
  return null;
};
