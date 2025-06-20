// import { useAppSelector } from "./useAppSelector";
// import {
//   selectAddressURL /*selectApiKey*/,
// } from "../features/Config/configSlice";
// import { smallCloudApi } from "../services/smallcloud";
// import { selectIsStreaming } from "../features/Chat";
// import { useGetCapsQuery } from "./useGetCapsQuery";
import { useQuery } from "urql";
import { NavTreeWantWorkspacesDocument } from "../../generated/documents";
// import { useAppDispatch } from "./useAppDispatch";
import { useEffect, useState } from "react";

// const NOT_SKIPPABLE_ADDRESS_URLS = [
//   "Refact",
//   "https://inference-backup.smallcloud.ai",
// ];

export const useGetUser = () => {
  // const maybeAddressURL = useAppSelector(selectAddressURL);

  const [isLoadingUserData, setIsLoadingUserData] = useState(true);
  const [userData] = useQuery({
    query: NavTreeWantWorkspacesDocument,
  });
  useEffect(() => {
    if (userData.data) {
      setIsLoadingUserData(false);
    }
  }, [userData.data]);

  // const addressURL = maybeAddressURL ? maybeAddressURL.trim() : "";
  // const maybeApiKey = useAppSelector(selectApiKey);
  // const { data: capsData } = useGetCapsQuery();
  // const supportsMetadata = capsData?.support_metadata;
  // const isStreaming = useAppSelector(selectIsStreaming);
  // const apiKey = maybeApiKey ?? "";
  // const isAddressURLALink =
  //   addressURL.startsWith("https://") || addressURL.startsWith("http://");

  // const request = smallCloudApi.useGetUserQuery(
  //   { apiKey, addressURL: addressURL },
  //   {
  //     skip:
  //       !(
  //         NOT_SKIPPABLE_ADDRESS_URLS.includes(addressURL) || isAddressURLALink
  //       ) ||
  //       isStreaming ||
  //       (supportsMetadata !== undefined && !supportsMetadata), // if it's enterprise, then skipping this request
  //     pollingInterval: 60 * 60 * 1000, // 1 hour
  //   },
  // );

  return {
    data: userData.data?.query_basic_stuff,
    isLoading: isLoadingUserData,
  };
};
