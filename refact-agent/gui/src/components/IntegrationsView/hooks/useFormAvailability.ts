import { useCallback } from "react";
import type {
  MCPArgs,
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
  setMCPArguments: React.Dispatch<React.SetStateAction<MCPArgs>>;
  setMCPEnvironmentVariables: React.Dispatch<
    React.SetStateAction<Record<string, string>>
  >;
  setHeaders: React.Dispatch<React.SetStateAction<Record<string, string>>>;
};

export const useFormAvailability = ({
  setAvailabilityValues,
  setConfirmationRules,
  setToolParameters,
  setMCPArguments,
  setMCPEnvironmentVariables,
  setHeaders,
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

  const handleMCPArguments = useCallback(
    (updatedArgs: MCPArgs) => {
      setMCPArguments(updatedArgs);
    },
    [setMCPArguments],
  );

  const handleMCPEnvironmentVariables = useCallback(
    (updatedEnvs: Record<string, string>) => {
      setMCPEnvironmentVariables(updatedEnvs);
    },
    [setMCPEnvironmentVariables],
  );

  const handleHeaders = useCallback(
    (updatedHeaders: Record<string, string>) => {
      setHeaders(updatedHeaders);
    },
    [setHeaders],
  );

  return {
    handleAvailabilityChange,
    handleConfirmationChange,
    handleToolParameters,
    handleMCPArguments,
    handleMCPEnvironmentVariables,
    handleHeaders,
  };
};
