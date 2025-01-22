import { useAppSelector } from "./useAppSelector";
import { selectAddressURL, selectApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";

const NOT_SKIPPABLE_ADDRESS_URLS = [
  "Refact",
  "https://inference-backup.smallcloud.ai",
];

export const useGetUser = () => {
  const maybeAddressURL = useAppSelector(selectAddressURL);
  const addressURL = maybeAddressURL ? maybeAddressURL.trim() : "";
  const maybeApiKey = useAppSelector(selectApiKey);
  const apiKey = maybeApiKey ?? "";
  const isAddressURLALink =
    addressURL.startsWith("https://") || addressURL.startsWith("http://");

  return smallCloudApi.useGetUserQuery(
    { apiKey, addressURL: addressURL },
    {
      skip: !(
        NOT_SKIPPABLE_ADDRESS_URLS.includes(addressURL) || isAddressURLALink
      ),
      refetchOnMountOrArgChange: true,
    },
  );
};
