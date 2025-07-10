import { useCallback } from "react";

import { LspChatMessage } from "../services/refact/chat";
import { formatMessagesForChat } from "../features/Chat/Thread/utils";
import { useAppDispatch } from "./useAppDispatch";
import { clearInformation } from "../features/Errors/informationSlice";

import { useGoToLink } from "./useGoToLink";
// import { newIntegrationChat } from "../features/Chat/Thread/actions";
// import { createThreadWitMultipleMessages } from "../services/graphql/graphqlThunks";
import {
  useExpertsAndModels,
  useModelsForExpert,
} from "../features/ExpertsAndModels";
import { useGetToolsLazyQuery } from "./useGetToolGroupsQuery";
import { Tool } from "../services/refact";
import { graphqlQueriesAndMutations } from "../services/graphql/graphqlThunks";

export function useSmartLinks() {
  const dispatch = useAppDispatch();
  // TODO: find the correct expert, don't use last used
  const { selectedExpert } = useExpertsAndModels();
  const { selectedModel } = useModelsForExpert();
  const [getTools, _] = useGetToolsLazyQuery();

  const [createThreadWitMultipleMessages, result] =
    graphqlQueriesAndMutations.useCreateThreadWitMultipleMessagesMutation();

  const { handleGoTo } = useGoToLink();
  const handleSmartLink = useCallback(
    async (
      sl_chat: LspChatMessage[],
      integrationName: string,
      integrationPath: string,
      integrationProject: string,
    ) => {
      const toolsRaw = (await getTools(undefined)).data ?? [];
      const tools = toolsRaw
        .reduce<Tool[]>((acc, cur) => {
          return [...acc, ...cur.tools];
        }, [])
        .filter((tool) => tool.enabled)
        .map((tool) => tool.spec);

      // TODO: change this to flexus format
      const messages = formatMessagesForChat(sl_chat);
      dispatch(clearInformation());
      // TODO: when in an integration, we should enable all patch like tool requests
      void createThreadWitMultipleMessages({
        messages,
        expertId: selectedExpert ?? "",
        model: selectedModel ?? "",
        tools: tools,
        integration: {
          name: integrationName,
          path: integrationPath,
          project: integrationProject,
        },
      });
    },
    [
      createThreadWitMultipleMessages,
      dispatch,
      getTools,
      selectedExpert,
      selectedModel,
    ],
  );

  return {
    handleSmartLink,
    handleGoTo,
    loading: result.isLoading,
  };
}
