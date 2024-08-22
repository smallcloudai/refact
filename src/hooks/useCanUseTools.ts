import { useMemo } from "react";
import {
  useAppSelector,
  useGetCapsQuery,
  useGetToolsQuery,
} from "../app/hooks";
import { selectModel } from "../features/Chat/chatThread";
import { CodeChatModel } from "../services/refact/caps";

export const useCanUseTools = () => {
  const capsRequest = useGetCapsQuery();
  const toolsRequest = useGetToolsQuery();
  const chatModel = useAppSelector(selectModel);

  const canUseTools = useMemo(() => {
    if (!capsRequest.data) return false;
    if (!toolsRequest.data) return false;
    if (toolsRequest.data.length === 0) return false;
    const modelName = chatModel || capsRequest.data.code_chat_default_model;

    if (!(modelName in capsRequest.data.code_chat_models)) return false;
    const model: CodeChatModel = capsRequest.data.code_chat_models[modelName];
    if ("supports_tools" in model && model.supports_tools) return true;
    return false;
  }, [capsRequest.data, toolsRequest.data, chatModel]);
  return canUseTools;
};
