import { useAppSelector } from "./useAppSelector";
import { selectAddressURL, selectApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";
// import { selectIsStreaming } from "../features/Chat";
// import { selectIsStreaming } from "../features/ThreadMessages";
// import { useGetCapsQuery } from "./useGetCapsQuery";

const NOT_SKIPPABLE_ADDRESS_URLS = [
  "Refact",
  "https://inference-backup.smallcloud.ai",
];

export const useGetUser = () => {
  const maybeAddressURL = useAppSelector(selectAddressURL);
  const addressURL = maybeAddressURL ? maybeAddressURL.trim() : "";
  const maybeApiKey = useAppSelector(selectApiKey);
  // const { data: capsData } = useGetCapsQuery();
  // const supportsMetadata = capsData?.support_metadata;
  // const isStreaming = useAppSelector(selectIsStreaming);
  const apiKey = maybeApiKey ?? "";
  const isAddressURLALink =
    addressURL.startsWith("https://") || addressURL.startsWith("http://");

  const request = smallCloudApi.useGetUserQuery(
    { apiKey, addressURL: addressURL },
    {
      skip: !(
        NOT_SKIPPABLE_ADDRESS_URLS.includes(addressURL) || isAddressURLALink
      ), // ||
      // isStreaming ||
      // (supportsMetadata !== undefined && !supportsMetadata), // if it's enterprise, then skipping this request
      pollingInterval: 60 * 60 * 1000, // 1 hour
    },
  );

  return request;
};
