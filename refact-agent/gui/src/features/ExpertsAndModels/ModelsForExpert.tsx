import React from "react";
import { Skeleton } from "@radix-ui/themes";
import { Select } from "../../components/Select";

import { useModelsForExpert } from "./useModelsForExpert";
export const ModelsForExpert: React.FC<{ disabled?: boolean }> = ({
  disabled,
}) => {
  const { modelsLoading, selectedModelOrDefault, selectModel, options } =
    useModelsForExpert();
  return (
    <Skeleton loading={modelsLoading}>
      <Select
        disabled={disabled}
        placeholder="Select Model"
        title="Models For Expert"
        value={selectedModelOrDefault}
        options={options}
        onChange={selectModel}
      />
    </Skeleton>
  );
};
