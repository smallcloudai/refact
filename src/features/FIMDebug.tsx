import React from "react";
import { FIMDebug as FIMDebugView } from "../components/FIMDebug";
import { useEventBysForFIMDebug } from "../hooks";

export const FIMDebug: React.FC = () => {
  const { state } = useEventBysForFIMDebug();
  if (state.data) return <FIMDebugView data={state.data} />;
  // TODO: add error view
  // TODO: add loading view
  return null;
};
