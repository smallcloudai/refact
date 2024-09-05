import { pingApi } from "../services/refact";

export const usePing = () => {
  return pingApi.usePingQuery(undefined);
};
