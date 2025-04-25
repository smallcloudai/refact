import { useCallback, useEffect, useMemo, useState } from "react";
import { selectThreadToolUse } from "../features/Chat/Thread/selectors";
import { useAppSelector, useGetCapsQuery, useAppDispatch } from ".";

import {
  getSelectedChatModel,
  setChatModel,
  setMaxNewTokens,
  setToolUse,
  ToolUse,
} from "../features/Chat";
import { DEFAULT_MAX_NEW_TOKENS } from "../services/refact";

// TODO: hard coded for now.
export const PAID_AGENT_LIST = [
  "gpt-4o",
  "claude-3-5-sonnet",
  "grok-2-1212",
  "grok-beta",
  "gemini-2.0-flash-exp",
  "claude-3-7-sonnet",
];

// TODO: hard coded for now. Unlimited usage models
export const UNLIMITED_PRO_MODELS_LIST = ["gpt-4o-mini"];

export function useCapsForToolUse() {
  const [wasAdjusted, setWasAdjusted] = useState(false);
  const caps = useGetCapsQuery();
  const toolUse = useAppSelector(selectThreadToolUse);
  const dispatch = useAppDispatch();

  const defaultCap = caps.data?.chat_default_model ?? "";

  const selectedModel = useAppSelector(getSelectedChatModel);

  const currentModel = selectedModel || defaultCap;

  const setCapModel = useCallback(
    (value: string) => {
      const model = caps.data?.chat_default_model === value ? "" : value;
      const action = setChatModel(model);
      dispatch(action);
      const tokens =
        caps.data?.chat_models[value]?.n_ctx ?? DEFAULT_MAX_NEW_TOKENS;
      dispatch(setMaxNewTokens(tokens));
    },
    [caps.data?.chat_default_model, caps.data?.chat_models, dispatch],
  );

  const isMultimodalitySupportedForCurrentModel = useMemo(() => {
    const models = caps.data?.chat_models;
    const item = models?.[currentModel];
    if (!item) return false;
    if (!item.supports_multimodality) return false;
    return true;
  }, [caps.data?.chat_models, currentModel]);

  const modelsSupportingTools = useMemo(() => {
    const models = caps.data?.chat_models ?? {};
    return Object.entries(models)
      .filter(([_, value]) => value.supports_tools)
      .map(([key]) => key);
  }, [caps.data?.chat_models]);

  const modelsSupportingAgent = useMemo(() => {
    const models = caps.data?.chat_models ?? {};
    return Object.entries(models)
      .filter(([_, value]) => value.supports_agent)
      .map(([key]) => key);
  }, [caps.data?.chat_models]);

  const usableModels = useMemo(() => {
    const models = caps.data?.chat_models ?? {};
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
  }, [caps.data?.chat_models, toolUse]);

  const usableModelsForPlan = useMemo(() => {
    // TODO: keep filtering logic for the future BYOK + Cloud (to show different providers)
    // if (user.data?.inference !== "FREE") return usableModels;
    // if (!usage.aboveUsageLimit && toolUse === "agent") return usableModels;
    return usableModels.map((model) => {
      // if (!PAID_AGENT_LIST.includes(model)) return model;

      return {
        value: model,
        disabled: false,
        textValue:
          // toolUse !== "agent" ? `${model} (Available in agent)` : undefined,
          model,
      };
    });
    // return usableModels;
  }, [
    // user.data?.inference,
    usableModels,
    // toolUse,
    // usage.aboveUsageLimit,
  ]);

  useEffect(() => {
    if (usableModelsForPlan.length > 0) {
      const models: string[] = usableModelsForPlan.map(
        (elem) => elem.textValue,
      );
      const toChange =
        models.find((elem) => currentModel === elem) ?? models[0];

      setCapModel(toChange);
    }
  }, [setCapModel, currentModel, usableModels, usableModelsForPlan]);

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
    uninitialized: caps.isUninitialized,
    data: caps.data,
  };
}
