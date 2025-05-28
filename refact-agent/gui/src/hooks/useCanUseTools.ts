import { useMemo } from "react";
import { useAppSelector } from "./useAppSelector";
import { useGetToolGroupsQuery } from "./useGetToolGroupsQuery";
import { useGetCapsQuery } from "./useGetCapsQuery";
import { selectModel } from "../features/Chat/Thread/selectors";
import { CodeChatModel } from "../services/refact/models";

export const useCanUseTools = () => {
  const capsRequest = useGetCapsQuery();
  const toolsRequest = useGetToolGroupsQuery();
  const chatModel = useAppSelector(selectModel);

  const loading = useMemo(() => {
    return capsRequest.isLoading || toolsRequest.isLoading;
  }, [capsRequest, toolsRequest]);

  // TODO: loading state.
  const canUseTools = useMemo(() => {
    if (!capsRequest.data) return false;
    if (!toolsRequest.data) return false;
    if (toolsRequest.data.length === 0) return false;
    const modelName = chatModel || capsRequest.data.chat_default_model;

    if (!(modelName in capsRequest.data.chat_models)) return false;
    const model: CodeChatModel = capsRequest.data.chat_models[modelName];
    if ("supports_tools" in model && model.supports_tools) return true;
    return false;
  }, [capsRequest.data, toolsRequest.data, chatModel]);
  return {
    canUseTools,
    loading,
  };
};
