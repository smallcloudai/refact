import { useCallback, useEffect, useMemo, useState } from "react";
import {
  selectChatId,
  selectThreadToolUse,
} from "../features/Chat/Thread/selectors";
import {
  useAppSelector,
  useGetCapsQuery,
  useAgentUsage,
  useAppDispatch,
  useGetUser,
} from ".";

import {
  getSelectedChatModel,
  setChatModel,
  updateMaximumContextTokens,
  setToolUse,
  ToolUse,
} from "../features/Chat";

// TODO: hard coded for now.
const PAID_AGENT_LIST = [
  "gpt-4o",
  "claude-3-5-sonnet",
  "grok-2-1212",
  "grok-beta",
  "gemini-2.0-flash-exp",
  "claude-3-7-sonnet",
];

export function useCapsForToolUse() {
  const [wasAdjusted, setWasAdjusted] = useState(false);
  const caps = useGetCapsQuery();
  const toolUse = useAppSelector(selectThreadToolUse);
  const chatId = useAppSelector(selectChatId);
  const usage = useAgentUsage();
  const user = useGetUser();
  const dispatch = useAppDispatch();

  const defaultCap = caps.data?.code_chat_default_model ?? "";
  const selectedModel = useAppSelector(getSelectedChatModel);

  const currentModel = selectedModel || defaultCap;

  const setCapModel = useCallback(
    (value: string) => {
      const model = caps.data?.code_chat_default_model === value ? "" : value;
      const action = setChatModel(model);
      dispatch(action);
    },
    [caps.data?.code_chat_default_model, dispatch],
  );

  const isMultimodalitySupportedForCurrentModel = useMemo(() => {
    const models = caps.data?.code_chat_models;
    const item = models?.[currentModel];
    if (!item) return false;
    if (!item.supports_multimodality) return false;
    return true;
  }, [caps.data?.code_chat_models, currentModel]);

  const modelsSupportingTools = useMemo(() => {
    const models = caps.data?.code_chat_models ?? {};
    return Object.entries(models)
      .filter(([_, value]) => value.supports_tools)
      .map(([key]) => key);
  }, [caps.data?.code_chat_models]);

  const modelsSupportingAgent = useMemo(() => {
    const models = caps.data?.code_chat_models ?? {};
    return Object.entries(models)
      .filter(([_, value]) => value.supports_agent)
      .map(([key]) => key);
  }, [caps.data?.code_chat_models]);

  const usableModels = useMemo(() => {
    const models = caps.data?.code_chat_models ?? {};
    const items = Object.entries(models).reduce<string[]>(
      (acc, [key, value]) => {
        if (toolUse === "explore" && value.supports_tools) {
          return [...acc, key];
        }
        if (toolUse === "agent" && value.supports_agent) return [...acc, key];
        if (toolUse === "quick") return [...acc, key];
        return acc;
      },
      [],
    );
    return items;
  }, [caps.data?.code_chat_models, toolUse]);

  const usableModelsForPlan = useMemo(() => {
    if (user.data?.inference !== "FREE") return usableModels;
    if (!usage.aboveUsageLimit && toolUse === "agent") return usableModels;
    return usableModels.map((model) => {
      if (!PAID_AGENT_LIST.includes(model)) return model;

      return {
        value: model,
        disabled: true,
        textValue:
          toolUse !== "agent" ? `${model} (Available in agent)` : undefined,
      };
    });
  }, [user.data?.inference, usableModels, usage.aboveUsageLimit, toolUse]);

  useEffect(() => {
    if (
      usableModelsForPlan.length > 0 &&
      usableModelsForPlan.some((elem) => typeof elem === "string") &&
      !usableModelsForPlan.includes(currentModel)
    ) {
      const models: string[] = usableModelsForPlan.filter(
        (elem): elem is string => typeof elem === "string",
      );
      const toChange =
        models.find((elem) => currentModel.startsWith(elem)) ??
        (models[0] || "");

      setCapModel(toChange);
    }
  }, [setCapModel, currentModel, usableModels, usableModelsForPlan]);

  useEffect(() => {
    const currentModelMaximumContextTokens =
      caps.data?.code_chat_models[currentModel]?.n_ctx;

    if (currentModelMaximumContextTokens) {
      const inputTokensLimit = parseInt(
        (currentModelMaximumContextTokens / 3).toFixed(0),
      );

      dispatch(
        updateMaximumContextTokens({
          chatId,
          value: inputTokensLimit,
        }),
      );
    }
  }, [dispatch, caps.data?.code_chat_models, chatId, currentModel]);

  useEffect(() => {
    const determineNewToolUse = (): ToolUse | null => {
      if (toolUse === "agent" && modelsSupportingAgent.length === 0) {
        return "explore";
      }
      if (toolUse === "explore" && modelsSupportingTools.length === 0) {
        return "quick";
      }
      return null;
    };

    const handleAutomaticToolUseChange = () => {
      if (!caps.isSuccess || wasAdjusted) return;

      const newToolUse = determineNewToolUse();
      if (newToolUse) {
        dispatch(setToolUse(newToolUse));
      }
      setWasAdjusted(true);
    };

    handleAutomaticToolUseChange();
  }, [
    dispatch,
    wasAdjusted,
    caps.isSuccess,
    toolUse,
    modelsSupportingAgent,
    modelsSupportingTools,
  ]);

  return {
    usableModels,
    usableModelsForPlan,
    currentModel,
    setCapModel,
    isMultimodalitySupportedForCurrentModel,
    loading: !caps.data && (caps.isFetching || caps.isLoading),
  };
}
