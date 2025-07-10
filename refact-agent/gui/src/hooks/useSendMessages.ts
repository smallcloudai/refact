import { useCallback } from "react";
import { useAppSelector } from "./useAppSelector";
import { useGetToolsLazyQuery } from "./useGetToolGroupsQuery";
import { FThreadMessageInput } from "../../generated/documents";
import { selectThreadEnd, selectAppSpecific } from "../features/ThreadMessages";
import {
  selectCurrentExpert,
  selectCurrentModel,
} from "../features/ExpertsAndModels";
import { Tool } from "../services/refact/tools";
import { selectAllImages } from "../features/AttachedImages";
import {
  UserMessage,
  UserMessageContentWithImage,
} from "../services/refact/types";
import { useIdForThread } from "./useIdForThread";
import { graphqlQueriesAndMutations } from "../services/graphql/graphqlThunks";

// TODO: since this is called twice it opens two sockets :/ move sendMessage and sendMultipleMessage to their own hooks

export function useSendMessages() {
  const leafMessage = useAppSelector(selectThreadEnd, {
    devModeChecks: { stabilityCheck: "never" },
  });

  const maybeFtId = useIdForThread();
  const appSpecific = useAppSelector(selectAppSpecific, {
    devModeChecks: { stabilityCheck: "never" },
  });

  const selectedExpert = useAppSelector(selectCurrentExpert);
  const selectedModel = useAppSelector(selectCurrentModel);
  const attachedImages = useAppSelector(selectAllImages);
  const [sendMessages, _sendMessagesResult] =
    graphqlQueriesAndMutations.useSendMessagesMutation();

  const [createThreadWitMultipleMessages] =
    graphqlQueriesAndMutations.useCreateThreadWitMultipleMessagesMutation();
  const [createThreadWithMessage] =
    graphqlQueriesAndMutations.useCreateThreadWithSingleMessageMutation();

  const [getTools, _getToolsResult] = useGetToolsLazyQuery();

  const maybeAddImagesToQuestion = useCallback(
    (question: string): UserMessage => {
      if (attachedImages.length === 0)
        return {
          ftm_role: "user" as const,
          ftm_content: question,
          checkpoints: [],
        };

      const images = attachedImages.reduce<UserMessageContentWithImage[]>(
        (acc, image) => {
          if (typeof image.content !== "string") return acc;
          return acc.concat({
            type: "image_url",
            image_url: { url: image.content },
          });
        },
        [],
      );

      if (images.length === 0)
        return { ftm_role: "user", ftm_content: question, checkpoints: [] };

      return {
        ftm_role: "user",
        ftm_content: [...images, { type: "text", text: question }],
        checkpoints: [],
      };
    },
    [attachedImages],
  );

  const sendMultipleMessages = useCallback(
    async (messages: { ftm_role: string; ftm_content: unknown }[]) => {
      const lspToolGroups = (await getTools(undefined)).data ?? [];
      const allTools = lspToolGroups.reduce<Tool[]>((acc, toolGroup) => {
        return [...acc, ...toolGroup.tools];
      }, []);
      const enabledTools = allTools.filter((tool) => tool.enabled);
      const specs = enabledTools.map((tool) => tool.spec);

      if (leafMessage.endAlt === 0 && leafMessage.endNumber === 0) {
        void createThreadWitMultipleMessages({
          messages,
          expertId: selectedExpert ?? "",
          model: selectedModel ?? "",
          tools: specs,
        });

        return;
      }

      const inputMessages = messages.map((message, index) => {
        return {
          ftm_alt: leafMessage.endAlt,
          ftm_app_specific: JSON.stringify(appSpecific),
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
            tools: specs,
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
      appSpecific,
      createThreadWitMultipleMessages,
      getTools,
      leafMessage.endAlt,
      leafMessage.endNumber,
      leafMessage.endPrevAlt,
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

      // TODO: add images

      if (leafMessage.endAlt === 0 && leafMessage.endNumber === 0) {
        void createThreadWithMessage({
          content,
          expertId: selectedExpert ?? "",
          model: selectedModel ?? "",
          tools: specs,
        });
        return;
      }
      const input: FThreadMessageInput = {
        ftm_alt: leafMessage.endAlt,
        ftm_app_specific: JSON.stringify(appSpecific),
        ftm_belongs_to_ft_id: maybeFtId ?? "", // ftId.ft_id,
        ftm_call_id: "",
        ftm_content: JSON.stringify(content),
        ftm_num: leafMessage.endNumber + 1,
        ftm_prev_alt: leafMessage.endPrevAlt,
        ftm_provenance: JSON.stringify(window.__REFACT_CHAT_VERSION__), // extra json data
        ftm_role: "user",
        ftm_tool_calls: "null", // optional
        ftm_usage: "null", // optional
        ftm_user_preferences: JSON.stringify({
          model: selectedModel ?? "",
          tools: specs,
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
      appSpecific,
      createThreadWithMessage,
      getTools,
      leafMessage.endAlt,
      leafMessage.endNumber,
      leafMessage.endPrevAlt,
      maybeFtId,
      selectedExpert,
      selectedModel,
      sendMessages,
    ],
  );

  return { sendMessage, sendMultipleMessages, maybeAddImagesToQuestion };
}
