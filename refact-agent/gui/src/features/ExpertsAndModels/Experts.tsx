import React, { useMemo } from "react";
import { Skeleton } from "@radix-ui/themes";
import { Select } from "../../components/Select";

import { useExpertsAndModels } from "./useExpertsAndModels";

export const ExpertSelect: React.FC<{ disabled?: boolean }> = ({
  disabled,
}) => {
  const { experts, expertsLoading, selectedExpert, onSelectExpert } =
    useExpertsAndModels();

  // should be handled in the slice
  const selectedExpertOrDefault = useMemo(() => {
    if (selectedExpert) return selectedExpert;
    if (experts && experts.experts_effective_list.length > 0)
      return experts.experts_effective_list[0].fexp_id;
    return undefined;
  }, [experts, selectedExpert]);

  return (
    <Skeleton loading={expertsLoading}>
      <Select
        disabled={disabled}
        onChange={onSelectExpert}
        value={selectedExpertOrDefault}
        options={
          experts?.experts_effective_list.map((expert) => ({
            value: expert.fexp_id,
            textValue: expert.fexp_name,
          })) ?? []
        }
      />
    </Skeleton>
  );
};
