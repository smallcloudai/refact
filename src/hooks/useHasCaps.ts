import { useGetCapsQuery } from "./useGetCapsQuery";

export const useHasCaps = () => {
  const maybeCaps = useGetCapsQuery();
  return !!maybeCaps.data;
};
