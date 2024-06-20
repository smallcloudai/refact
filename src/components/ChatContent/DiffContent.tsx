import React from "react";
import { Text } from "@radix-ui/themes";
import { DiffAction } from "../../events";

export const DiffContent: React.FC<{ diffs: DiffAction[] }> = ({ diffs }) => {
  return <Text>{JSON.stringify(diffs, null, 2)}</Text>;
};
