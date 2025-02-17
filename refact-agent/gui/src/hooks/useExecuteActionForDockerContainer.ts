import { dockerApi } from "../services/refact";

export const useExecuteActionForDockerContainerMutation = () => {
  return dockerApi.useExecuteActionForDockerContainerMutation();
};
