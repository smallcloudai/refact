import { useCallback, useEffect, useMemo } from "react";

import {
  useAppDispatch,
  useAppSelector,
  useGetToolsLazyQuery,
} from "../../hooks";
import { selectCurrentPage } from "../../features/Pages/pagesSlice";
import {
  messagesSub,
  createMessage,
  createThreadWithMessage,
  pauseThreadThunk,
  createThreadWitMultipleMessages,
} from "../../services/graphql/graphqlThunks";
import { FThreadMessageInput } from "../../../generated/documents";
import {
  isThreadEmpty,
  selectThreadId,
  selectThreadEnd,
  selectAppSpecific,
  selectIsStreaming,
  selectIsWaiting,
} from "../../features/ThreadMessages";
import {
  selectCurrentExpert,
  selectCurrentModel,
} from "../../features/ExpertsAndModels";
import { Tool } from "../../services/refact/tools";
import { selectAllImages } from "../../features/AttachedImages";
import {
  UserMessage,
  UserMessageContentWithImage,
} from "../../services/refact/types";

export function useMessageSubscription() {
  const dispatch = useAppDispatch();
  const leafMessage = useAppSelector(selectThreadEnd, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const isEmpty = useAppSelector(isThreadEmpty, {
    devModeChecks: { stabilityCheck: "never" },
  });
  const maybeFtId = useIdForThread();
  const appSpecific = useAppSelector(selectAppSpecific, {
    devModeChecks: { stabilityCheck: "never" },
  });

  const selectedExpert = useAppSelector(selectCurrentExpert);
  const selectedModel = useAppSelector(selectCurrentModel);
  const attachedImages = useAppSelector(selectAllImages);

  useEffect(() => {
    if (!maybeFtId) return;
    const thunk = dispatch(
      messagesSub({ ft_id: maybeFtId, want_deltas: true }),
    );
    return () => {
      thunk.abort();
    };
  }, [dispatch, isEmpty, maybeFtId]);

  // TODO: the user should be able to configure this
  const [getTools, _] = useGetToolsLazyQuery();

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
        const action = createThreadWitMultipleMessages({
          messages,
          expertId: selectedExpert ?? "",
          model: selectedModel ?? "",
          tools: specs,
        });

        void dispatch(action);
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

      const action = createMessage({
        input: {
          ftm_belongs_to_ft_id: maybeFtId ?? "",
          messages: inputMessages,
        },
      });
      void dispatch(action);
    },
    [
      appSpecific,
      dispatch,
      getTools,
      leafMessage.endAlt,
      leafMessage.endNumber,
      leafMessage.endPrevAlt,
      maybeFtId,
      selectedExpert,
      selectedModel,
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

      if (leafMessage.endAlt === 0 && leafMessage.endNumber === 0) {
        void dispatch(
          createThreadWithMessage({
            content,
            expertId: selectedExpert ?? "",
            model: selectedModel ?? "",
            tools: specs,
          }),
        );
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
      void dispatch(
        createMessage({
          input: {
            ftm_belongs_to_ft_id: maybeFtId ?? "",
            messages: [input],
          },
        }),
      );
    },
    [
      appSpecific,
      dispatch,
      getTools,
      leafMessage.endAlt,
      leafMessage.endNumber,
      leafMessage.endPrevAlt,
      maybeFtId,
      selectedExpert,
      selectedModel,
    ],
  );

  return { sendMessage, sendMultipleMessages, maybeAddImagesToQuestion };
}

// TODO: id comes from the route or backend when creating a new thread
export const useIdForThread = () => {
  const route = useAppSelector(selectCurrentPage);
  const ftId = useAppSelector(selectThreadId);

  const idInfo = useMemo(() => {
    if (ftId) return ftId;
    if (route && "ft_id" in route && route.ft_id) {
      return route.ft_id;
    }
    return null;
  }, [route, ftId]);

  return idInfo;
};

export const usePauseThread = () => {
  const dispatch = useAppDispatch();
  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const threadId = useAppSelector(selectThreadId);

  const shouldShow = useMemo(() => {
    if (!threadId) return false;
    return isStreaming || isWaiting;
  }, [threadId, isStreaming, isWaiting]);

  const handleStop = useCallback(() => {
    if (!threadId) return;
    void dispatch(pauseThreadThunk({ id: threadId }));
  }, [dispatch, threadId]);

  return { shouldShow, handleStop };
};
