import { useAppSelector } from "../app/hooks";
import { selectApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";

export const useGetUser = () => {
  const maybeApiKey = useAppSelector(selectApiKey);
  const apiKey = maybeApiKey ?? "";
  return smallCloudApi.useGetUserQuery(apiKey, { skip: !maybeApiKey });
};
