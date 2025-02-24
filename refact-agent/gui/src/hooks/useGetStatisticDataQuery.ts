import { statisticsApi } from "../services/refact/statistics";
import { useGetPing } from "./useGetPing";

export const useGetStatisticDataQuery = () => {
  const ping = useGetPing();
  const skip = !ping.data;
  return statisticsApi.useGetStatisticDataQuery(undefined, { skip });
};
