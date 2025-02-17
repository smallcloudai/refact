import { pingApi } from "../services/refact";

export const useGetPing = () => {
  return pingApi.usePingQuery(undefined);
};
