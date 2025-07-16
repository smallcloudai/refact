import { dockerApi } from "../services/refact/docker";

export const useExecuteActionForDockerContainerMutation = () => {
  return dockerApi.useExecuteActionForDockerContainerMutation();
};
