import { statisticsApi } from "../services/refact/statistics";

export const useGetStatisticDataQuery = () => {
  return statisticsApi.useGetStatisticDataQuery(undefined);
};
