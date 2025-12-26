import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  isCommitLink,
  isPostChatLink,
  isUserMessage,
  linksApi,
  type ChatLink,
} from "..//services/refact";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import { useGetCapsQuery } from "./useGetCapsQuery";
import { useChatActions } from "./useChatActions";
import {
  chatModeToLspMode,
  selectAreFollowUpsEnabled,
  selectChatId,
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectModel,
  selectThreadMode,
  setIncreaseMaxTokens,
  setIntegrationData,
  setIsNewChatSuggested,
} from "../features/Chat";
import { useGoToLink } from "./useGoToLink";
import { setError } from "../features/Errors/errorsSlice";
import { setInformation } from "../features/Errors/informationSlice";
import { debugIntegrations, debugRefact } from "../debugConfig";
import { telemetryApi } from "../services/refact/telemetry";
import { isAbsolutePath } from "../utils";

export function useGetLinksFromLsp() {
  const dispatch = useAppDispatch();

  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const messages = useAppSelector(selectMessages);
  const chatId = useAppSelector(selectChatId);
  const maybeIntegration = useAppSelector(selectIntegration);
  const threadMode = useAppSelector(selectThreadMode);
  const areFollowUpsEnabled = useAppSelector(selectAreFollowUpsEnabled);

  // TODO: add the model
  const caps = useGetCapsQuery();

  const model = useAppSelector(selectModel) || caps.data?.chat_default_model;

  const unCalledTools = React.useMemo(() => {
    if (messages.length === 0) return false;
    const last = messages[messages.length - 1];
    //TODO: handle multiple tool calls in last assistant message
    if (last.role !== "assistant") return false;
    const maybeTools = last.tool_calls;
    if (maybeTools && maybeTools.length > 0) return true;
    return false;
  }, [messages]);

  const skipLinksRequest = useMemo(() => {
    const lastMessageIsUserMessage =
      messages.length > 0 && isUserMessage(messages[messages.length - 1]);
    if (!model) return true;
    if (!caps.data) return true;
    return (
      !areFollowUpsEnabled ||
      isStreaming ||
      isWaiting ||
      unCalledTools ||
      lastMessageIsUserMessage
    );
  }, [
    caps.data,
    areFollowUpsEnabled,
    isStreaming,
    isWaiting,
    messages,
    model,
    unCalledTools,
  ]);

  const linksResult = linksApi.useGetLinksForChatQuery(
    {
      chat_id: chatId,
      messages,
      model: model ?? "",
      mode: chatModeToLspMode({ defaultMode: threadMode }),
      current_config_file: maybeIntegration?.path,
    },
    { skip: skipLinksRequest },
  );

  useEffect(() => {
    if (linksResult.data?.new_chat_suggestion) {
      dispatch(
        setIsNewChatSuggested({
          chatId,
          value: linksResult.data.new_chat_suggestion,
        }),
      );
    }
  }, [dispatch, linksResult.data, chatId]);

  return linksResult;
}

export function useLinksFromLsp() {
  const dispatch = useAppDispatch();
  const { handleGoTo } = useGoToLink();
  const { submit, setParams } = useChatActions();

  const [applyCommit, _applyCommitResult] = linksApi.useSendCommitMutation();

  const [sendTelemetryEvent] =
    telemetryApi.useLazySendTelemetryChatEventQuery();

  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const messages = useAppSelector(selectMessages);
  const maybeIntegration = useAppSelector(selectIntegration);

  const unCalledTools = React.useMemo(() => {
    if (messages.length === 0) return false;
    const last = messages[messages.length - 1];
    //TODO: handle multiple tool calls in last assistant message
    if (last.role !== "assistant") return false;
    const maybeTools = last.tool_calls;
    if (maybeTools && maybeTools.length > 0) return true;
    return false;
  }, [messages]);

  // TODO: think of how to avoid batching and this useless state
  const [pendingIntegrationGoto, setPendingIntegrationGoto] = useState<
    string | null
  >(null);

  useEffect(() => {
    if (
      maybeIntegration?.shouldIntermediatePageShowUp !== undefined &&
      pendingIntegrationGoto
    ) {
      handleGoTo({ goto: pendingIntegrationGoto });
      setPendingIntegrationGoto(null);
    }
  }, [pendingIntegrationGoto, handleGoTo, maybeIntegration]);

  const handleLinkAction = useCallback(
    (link: ChatLink) => {
      if (!("link_action" in link)) return;
      void sendTelemetryEvent({
        scope: `handleLinkAction/${link.link_action}`,
        success: true,
        error_message: "",
      });

      if (
        link.link_action === "goto" &&
        "link_goto" in link &&
        link.link_goto !== undefined
      ) {
        const [action, ...payloadParts] = link.link_goto.split(":");
        const payload = payloadParts.join(":");
        if (action.toLowerCase() === "settings") {
          debugIntegrations(
            `[DEBUG]: this goto is integrations one, dispatching integration data`,
          );
          if (!isAbsolutePath(payload)) {
            dispatch(
              setIntegrationData({
                name: payload,
                path: undefined,
                shouldIntermediatePageShowUp: payload !== "DEFAULT",
              }),
            );
          } else {
            dispatch(
              setIntegrationData({
                path: payload,
                shouldIntermediatePageShowUp: false,
              }),
            );
          }
          setPendingIntegrationGoto(link.link_goto);
        }
        handleGoTo({
          goto: link.link_goto,
        });
        return;
      }

      if (link.link_action === "patch-all") {
        // TBD: smart links for patches
        // void applyPatches(messages).then(() => {
        //   if ("link_goto" in link) {
        //     handleGoTo({ goto: link.link_goto });
        //   }
        // });
        if ("link_goto" in link) {
          handleGoTo({ goto: link.link_goto });
        }
        return;
      }

      if (link.link_action === "follow-up") {
        void submit(link.link_text);
        return;
      }

      if (link.link_action === "summarize-project") {
        if ("link_summary_path" in link && link.link_summary_path) {
          dispatch(setIntegrationData({ path: link.link_summary_path }));
        }
        // Set mode then send message
        void setParams({ mode: "PROJECT_SUMMARY" }).then(() => {
          void submit(link.link_text);
        });
        return;
      }

      // TBD: It should be safe to remove this now?
      if (link.link_action === "regenerate-with-increased-context-size") {
        dispatch(setIncreaseMaxTokens(true));
        return;
      }

      if (isCommitLink(link)) {
        void applyCommit(link.link_payload)
          .unwrap()
          .then((res) => {
            const commits = res.commits_applied;

            if (commits.length > 0) {
              const commitInfo = commits
                .map((commit, index) => `${index + 1}: ${commit.project_name}`)
                .join("\n");
              const message = `Successfully committed: ${commits.length}\n${commitInfo}`;
              dispatch(setInformation(message));
            }

            const errors = res.error_log
              .map((err, index) => {
                return `${index + 1}: ${err.project_name}\n${
                  err.project_path
                }\n${err.error_message}`;
              })
              .join("\n");
            if (errors) {
              dispatch(setError(`Commit errors: ${errors}`));
            }
          });

        return;
      }

      if (isPostChatLink(link)) {
        dispatch(
          setIntegrationData({
            path: link.link_payload.chat_meta.current_config_file,
          }),
        );
        debugRefact(`[DEBUG]: link messages: `, link.link_payload.messages);
        const lastMsg = link.link_payload.messages[link.link_payload.messages.length - 1];
        if (lastMsg.role === "user") {
          const content = typeof lastMsg.content === "string"
            ? lastMsg.content
            : "";
          void setParams({ mode: link.link_payload.chat_meta.chat_mode }).then(() => {
            void submit(content);
          });
        }
        return;
      }

      // eslint-disable-next-line no-console
      console.warn(`unknown action: ${JSON.stringify(link)}`);
    },
    [applyCommit, dispatch, handleGoTo, sendTelemetryEvent, submit, setParams],
  );

  const linksResult = useGetLinksFromLsp();

  return {
    linksResult,
    handleLinkAction,
    streaming: isWaiting || isStreaming || unCalledTools,
  };
}
