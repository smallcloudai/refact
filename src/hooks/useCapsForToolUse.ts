import { useCallback, useEffect, useMemo } from "react";
import { selectThreadToolUse } from "../features/Chat/Thread/selectors";
import {
  useAppSelector,
  useGetCapsQuery,
  useGetUser,
  useAgentUsage,
  useAppDispatch,
} from ".";

import { getSelectedChatModel, setChatModel } from "../features/Chat";

// TODO: hard coded for now.
const PAID_AGENT_LIST = ["gpt-4o", "claude-3-5-sonnet"];

export function useCapsForToolUse() {
  const caps = useGetCapsQuery();
  const toolUse = useAppSelector(selectThreadToolUse);
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

  const usableModels = useMemo(() => {
    const models = caps.data?.code_chat_models ?? {};
    const items = Object.entries(models).reduce<string[]>(
      (acc, [key, value]) => {
        if (toolUse !== "agent") return [...acc, key];
        if (value.supports_agent) return [...acc, key];
        return acc;
      },
      [],
    );
    return items;
  }, [caps.data?.code_chat_models, toolUse]);

  const usableModelsForPlan = useMemo(() => {
    if (user.data?.inference === "PRO") return usableModels;
    if (!usage.aboveUsageLimit) return usableModels;
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
  }, [currentModel, setCapModel, usableModels, usableModelsForPlan]);

  return {
    usableModels,
    usableModelsForPlan,
    currentModel,
    setCapModel,
    loading: !caps.data && (caps.isFetching || caps.isLoading),
  };
}
