import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  diffApi,
  isUserMessage,
  linksApi,
  type ChatLink,
} from "..//services/refact";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import { useGetCapsQuery } from "./useGetCapsQuery";
import { useSendChatRequest } from "./useSendChatRequest";
import {
  selectChatId,
  selectIntegration,
  selectIsStreaming,
  selectIsWaiting,
  selectMessages,
  selectModel,
  selectThreadMode,
  selectThreadToolUse,
  setIntegrationData,
} from "../features/Chat";
import { useGoToLink } from "./useGoToLink";
import { debugIntegrations } from "../debugConfig";

export function useLinksFromLsp() {
  const dispatch = useAppDispatch();
  const { handleGoTo } = useGoToLink();
  const { submit } = useSendChatRequest();

  const [applyPatches, _applyPatchesResult] =
    diffApi.useApplyAllPatchesInMessagesMutation();

  const isStreaming = useAppSelector(selectIsStreaming);
  const isWaiting = useAppSelector(selectIsWaiting);
  const messages = useAppSelector(selectMessages);
  const chatId = useAppSelector(selectChatId);
  const maybeIntegration = useAppSelector(selectIntegration);
  const threadMode = useAppSelector(selectThreadMode);
  const toolUse = useAppSelector(selectThreadToolUse);

  // TODO: add the model
  const caps = useGetCapsQuery();

  const model =
    useAppSelector(selectModel) || caps.data?.code_chat_default_model;

  const unCalledTools = React.useMemo(() => {
    if (messages.length === 0) return false;
    const last = messages[messages.length - 1];
    //TODO: handle multiple tool calls in last assistant message
    if (last.role !== "assistant") return false;
    const maybeTools = last.tool_calls;
    if (maybeTools && maybeTools.length > 0) return true;
    return false;
  }, [messages]);

  const [pendingIntegrationGoto, setPendingIntegrationGoto] = useState<
    string | null
  >(null);

  useEffect(() => {
    if (
      typeof maybeIntegration?.shouldIntermediatePageShowUp !== "undefined" &&
      pendingIntegrationGoto
    ) {
      handleGoTo({ goto: pendingIntegrationGoto });
      setPendingIntegrationGoto(null);
    }
  }, [pendingIntegrationGoto, handleGoTo, maybeIntegration]);

  const handleLinkAction = useCallback(
    (link: ChatLink) => {
      if (!("action" in link)) return;
      if (link.action === "goto" && "goto" in link) {
        const [action, payload] = link.goto.split(":");
        if (action.toLowerCase() === "settings") {
          debugIntegrations(
            `[DEBUG]: this goto is integrations one, dispatching integration data`,
          );
          dispatch(
            setIntegrationData({
              name: payload,
              shouldIntermediatePageShowUp: payload !== "DEFAULT",
            }),
          );
          setPendingIntegrationGoto(link.goto);
        }
        handleGoTo({
          goto: link.goto,
        });
        return;
      }

      if (link.action === "patch-all") {
        void applyPatches(messages).then(() => {
          if ("goto" in link) {
            handleGoTo({ goto: link.goto });
          }
        });
        return;
      }

      if (link.action === "follow-up") {
        submit(link.text);
        return;
      }

      if (link.action === "summarize-project") {
        if ("current_config_file" in link && link.current_config_file) {
          dispatch(setIntegrationData({ path: link.current_config_file }));
          // set the integration data
        }
        submit(link.text, "PROJECT_SUMMARY");
        return;
      }

      // if (link.action === "commit") {
      //   // TODO: there should be an endpoint for this
      //   void applyPatches(messages).then(() => {
      //     if ("goto" in link && link.goto) {
      //       handleGoTo(link.goto);
      //     }
      //   });

      //   return;
      // }

      // eslint-disable-next-line no-console
      console.warn(`unknown action: ${JSON.stringify(link)}`);
    },
    [applyPatches, dispatch, handleGoTo, messages, submit],
  );

  const skipLinksRequest = useMemo(() => {
    const lastMessageIsUserMessage =
      messages.length > 0 && isUserMessage(messages[messages.length - 1]);
    if (!model) return true;
    if (!caps.data) return true;
    if (toolUse !== "agent") return true;
    return (
      isStreaming || isWaiting || unCalledTools || lastMessageIsUserMessage
    );
  }, [
    caps.data,
    isStreaming,
    isWaiting,
    messages,
    model,
    toolUse,
    unCalledTools,
  ]);

  const linksResult = linksApi.useGetLinksForChatQuery(
    {
      chat_id: chatId,
      messages,
      model: model ?? "",
      mode: threadMode, // TODO: Changing thread mode invalidates the cache.
      current_config_file: maybeIntegration?.path,
    },
    { skip: skipLinksRequest },
  );

  return {
    linksResult,
    handleLinkAction,
    streaming: isWaiting || isStreaming || unCalledTools,
  };
}
