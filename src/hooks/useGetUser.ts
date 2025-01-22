import { useAppSelector } from "./useAppSelector";
import { selectAddressURL, selectApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";

const NOT_SKIPPABLE_ADDRESS_URLS = [
  "Refact",
  "https://inference-backup.smallcloud.ai",
];

export const useGetUser = () => {
  const addressURL = useAppSelector(selectAddressURL);
  const maybeApiKey = useAppSelector(selectApiKey);
  const apiKey = maybeApiKey ?? "";
  return smallCloudApi.useGetUserQuery(
    { apiKey, addressURL: addressURL?.trim() },
    {
      skip: !NOT_SKIPPABLE_ADDRESS_URLS.includes(addressURL?.trim() ?? ""),
      refetchOnMountOrArgChange: true,
    },
  );
};
