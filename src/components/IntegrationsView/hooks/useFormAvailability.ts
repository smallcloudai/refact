import { useCallback } from "react";
import type {
  ToolConfirmation,
  ToolParameterEntity,
} from "../../../services/refact";

type UseFormAvailabilityProps = {
  setAvailabilityValues: React.Dispatch<
    React.SetStateAction<Record<string, boolean>>
  >;
  setConfirmationRules: React.Dispatch<React.SetStateAction<ToolConfirmation>>;
  setToolParameters: React.Dispatch<
    React.SetStateAction<ToolParameterEntity[] | null>
  >;
};

export const useFormAvailability = ({
  setAvailabilityValues,
  setConfirmationRules,
  setToolParameters,
}: UseFormAvailabilityProps) => {
  const handleAvailabilityChange = useCallback(
    (fieldName: string, value: boolean) => {
      setAvailabilityValues((prev) => ({ ...prev, [fieldName]: value }));
    },
    [setAvailabilityValues],
  );

  const handleConfirmationChange = useCallback(
    (fieldName: string, values: string[]) => {
      setConfirmationRules((prev) => ({
        ...prev,
        [fieldName as keyof ToolConfirmation]: values,
      }));
    },
    [setConfirmationRules],
  );

  const handleToolParameters = useCallback(
    (value: ToolParameterEntity[]) => {
      setToolParameters(value);
    },
    [setToolParameters],
  );

  return {
    handleAvailabilityChange,
    handleConfirmationChange,
    handleToolParameters,
  };
};
