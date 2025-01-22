import { useAppSelector } from "./useAppSelector";
import { selectAddressURL, selectApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";

const NOT_SKIPPABLE_ADDRESS_URLS = [
  "Refact",
  "https://inference-backup.smallcloud.ai",
];

export const useGetUser = () => {
  const addressURL = useAppSelector(selectAddressURL);
  console.log(`[DEBUG]: addressURL: `, addressURL);
  const maybeApiKey = useAppSelector(selectApiKey);
  const apiKey = maybeApiKey ?? "";
  return smallCloudApi.useGetUserQuery(apiKey, {
    skip: !NOT_SKIPPABLE_ADDRESS_URLS.includes(addressURL ?? ""),
  });
};
