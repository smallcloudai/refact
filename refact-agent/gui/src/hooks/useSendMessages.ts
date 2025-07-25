import { useCallback } from "react";
import { useAppSelector } from "./useAppSelector";
import { useGetToolsLazyQuery } from "./useGetToolGroupsQuery";
import { FThreadMessageInput } from "../../generated/documents";
import { selectThreadEnd } from "../features/ThreadMessages/threadMessagesSlice";
import {
  selectCurrentExpert,
  selectCurrentModel,
} from "../features/ExpertsAndModels/expertsSlice";
import { Tool } from "../services/refact/tools";
import { useIdForThread } from "./useIdForThread";
import { graphqlQueriesAndMutations } from "../services/graphql/queriesAndMutationsApi";
import { useAttachImages } from "./useAttachImages";

// TODO: since this is called twice it opens two sockets :/ move sendMessage and sendMultipleMessage to their own hooks

export function useSendMessages() {
  const leafMessage = useAppSelector(selectThreadEnd, {
    devModeChecks: { stabilityCheck: "never" },
  });

  const maybeFtId = useIdForThread();

  const selectedExpert = useAppSelector(selectCurrentExpert);
  const selectedModel = useAppSelector(selectCurrentModel);
  const [sendMessages, _sendMessagesResult] =
    graphqlQueriesAndMutations.useSendMessagesMutation();

  const [createThreadWitMultipleMessages] =
    graphqlQueriesAndMutations.useCreateThreadWitMultipleMessagesMutation();
  const [createThreadWithMessage] =
    graphqlQueriesAndMutations.useCreateThreadWithSingleMessageMutation();

  const [getTools, _getToolsResult] = useGetToolsLazyQuery();

  const { maybeAddImagesToMessages, maybeAddImagesToContent } =
    useAttachImages();

  const sendMultipleMessages = useCallback(
    async (messages: { ftm_role: string; ftm_content: unknown }[]) => {
      const lspToolGroups = (await getTools(undefined)).data ?? [];
      const allTools = lspToolGroups.reduce<Tool[]>((acc, toolGroup) => {
        return [...acc, ...toolGroup.tools];
      }, []);
      const enabledTools = allTools.filter((tool) => tool.enabled);
      const specs = enabledTools.map((tool) => tool.spec);
      const maybeMessageWithImages = maybeAddImagesToMessages(messages);

      if (leafMessage.endAlt === 0 && leafMessage.endNumber === 0) {
        void createThreadWitMultipleMessages({
          messages: maybeMessageWithImages,
          expertId: selectedExpert ?? "",
          model: selectedModel ?? "",
          tools: specs,
        });

        return;
      }

      const inputMessages = maybeMessageWithImages.map((message, index) => {
        return {
          ftm_alt: leafMessage.endAlt,
          ftm_belongs_to_ft_id: maybeFtId ?? "", // ftId.ft_id,
          ftm_call_id: "",
          ftm_content: JSON.stringify(message.ftm_content),
          ftm_num: leafMessage.endNumber + index + 1,
          ftm_prev_alt: leafMessage.endPrevAlt,
          ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
          ftm_role: message.ftm_role,
          ftm_tool_calls: "null", // optional
          ftm_usage: "null", // optional
          ftm_user_preferences: JSON.stringify({
            model: selectedModel ?? "",
          }),
        };
      });

      void sendMessages({
        input: {
          ftm_belongs_to_ft_id: maybeFtId ?? "",
          messages: inputMessages,
        },
      });
    },
    [
      createThreadWitMultipleMessages,
      getTools,
      leafMessage.endAlt,
      leafMessage.endNumber,
      leafMessage.endPrevAlt,
      maybeAddImagesToMessages,
      maybeFtId,
      selectedExpert,
      selectedModel,
      sendMessages,
    ],
  );

  const sendMessage = useCallback(
    async (content: string) => {
      const lspToolGroups = (await getTools(undefined)).data ?? [];
      const allTools = lspToolGroups.reduce<Tool[]>((acc, toolGroup) => {
        return [...acc, ...toolGroup.tools];
      }, []);
      const enabledTools = allTools.filter((tool) => tool.enabled);
      const specs = enabledTools.map((tool) => tool.spec);

      const contentWithImage = maybeAddImagesToContent(content);

      if (leafMessage.endAlt === 0 && leafMessage.endNumber === 0) {
        void createThreadWithMessage({
          content: contentWithImage,
          expertId: selectedExpert ?? "",
          model: selectedModel ?? "",
          tools: specs,
        });
        return;
      }
      const input: FThreadMessageInput = {
        ftm_alt: leafMessage.endAlt,
        ftm_belongs_to_ft_id: maybeFtId ?? "", // ftId.ft_id,
        ftm_call_id: "",
        ftm_content: JSON.stringify(contentWithImage),
        ftm_num: leafMessage.endNumber + 1,
        ftm_prev_alt: leafMessage.endPrevAlt,
        ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
        ftm_role: "user",
        ftm_tool_calls: "null", // optional
        ftm_usage: "null", // optional
        ftm_user_preferences: JSON.stringify({
          model: selectedModel ?? "",
        }),
      };
      // TODO: this will need more info
      void sendMessages({
        input: {
          ftm_belongs_to_ft_id: maybeFtId ?? "",
          messages: [input],
        },
      });
    },
    [
      createThreadWithMessage,
      getTools,
      leafMessage.endAlt,
      leafMessage.endNumber,
      leafMessage.endPrevAlt,
      maybeAddImagesToContent,
      maybeFtId,
      selectedExpert,
      selectedModel,
      sendMessages,
    ],
  );

  return {
    sendMessage,
    sendMultipleMessages,
  };
}
