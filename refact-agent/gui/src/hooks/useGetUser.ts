import { useEffect } from "react";
import { useAppSelector } from "./useAppSelector";
import { selectAddressURL, selectApiKey } from "../features/Config/configSlice";
import { smallCloudApi } from "../services/smallcloud";
import { setInitialAgentUsage } from "../features/AgentUsage/agentUsageSlice";
import { useAppDispatch } from "./useAppDispatch";
import { selectIsStreaming } from "../features/Chat";

const NOT_SKIPPABLE_ADDRESS_URLS = [
  "Refact",
  "https://inference-backup.smallcloud.ai",
];

export const useGetUser = () => {
  const dispatch = useAppDispatch();
  const maybeAddressURL = useAppSelector(selectAddressURL);
  const addressURL = maybeAddressURL ? maybeAddressURL.trim() : "";
  const maybeApiKey = useAppSelector(selectApiKey);
  const isStreaming = useAppSelector(selectIsStreaming);
  const apiKey = maybeApiKey ?? "";
  const isAddressURLALink =
    addressURL.startsWith("https://") || addressURL.startsWith("http://");

  const request = smallCloudApi.useGetUserQuery(
    { apiKey, addressURL: addressURL },
    {
      skip:
        !(
          NOT_SKIPPABLE_ADDRESS_URLS.includes(addressURL) || isAddressURLALink
        ) || isStreaming,
      refetchOnMountOrArgChange: true,
    },
  );

  useEffect(() => {
    if (request.data && !isStreaming) {
      const action = setInitialAgentUsage({
        agent_usage: request.data.refact_agent_request_available,
        agent_max_usage_amount: request.data.refact_agent_max_request_num,
      });
      dispatch(action);
    }
  }, [dispatch, request.data, isStreaming]);

  return request;
};
