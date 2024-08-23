import { useAppSelector } from "../app/hooks";
import { selectAddressURL, selectApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";

export const useGetUser = () => {
  const addressURL = useAppSelector(selectAddressURL);
  const maybeApiKey = useAppSelector(selectApiKey);
  const apiKey = maybeApiKey ?? "";
  return smallCloudApi.useGetUserQuery(apiKey, {
    skip: !maybeApiKey || addressURL !== "Refact",
  });
};
