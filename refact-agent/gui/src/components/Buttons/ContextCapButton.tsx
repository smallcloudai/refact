import React, { useCallback, useMemo } from "react";
import { useAppDispatch, useAppSelector, useGetCapsQuery } from "../../hooks";
import {
  selectChatId,
  selectContextTokensCap,
  selectModel,
  setContextTokensCap,
} from "../../features/Chat/Thread";

import { Select, type SelectProps } from "../Select";
import { Skeleton } from "@radix-ui/themes";

const formatContextSize = (tokens: number): string => {
  if (tokens >= 1000000) {
    const m = tokens / 1000000;
    return Number.isInteger(m) ? `${m}M` : `${Math.round(m)}M`;
  }
  if (tokens >= 1000) {
    const k = tokens / 1000;
    return Number.isInteger(k) ? `${k}K` : `${Math.round(k)}K`;
  }
  return String(tokens);
};

const FIXED_OPTIONS = [
  256 * 1024, // 256K
  200 * 1024, // 200K
  128 * 1024, // 128K
  64 * 1024, // 64K
  32 * 1024, // 32K
  16 * 1024, // 16K (minimum)
];

const MIN_CONTEXT_CAP = 16 * 1024; // 16K minimum

export const ContextCapButton: React.FC = () => {
  const dispatch = useAppDispatch();
  const chatId = useAppSelector(selectChatId);
  const contextCap = useAppSelector(selectContextTokensCap);
  const threadModel = useAppSelector(selectModel);
  const capsQuery = useGetCapsQuery();

  // Derive maxTokens directly from caps data and current model
  // This avoids timing issues with threadMaxTokens state updates
  const maxTokens = useMemo(() => {
    if (!capsQuery.data) return undefined;

    // Use thread model if available in caps
    const modelToUse =
      threadModel && threadModel in capsQuery.data.chat_models
        ? threadModel
        : capsQuery.data.chat_default_model;

    if (modelToUse in capsQuery.data.chat_models) {
      return capsQuery.data.chat_models[modelToUse].n_ctx;
    }

    return undefined;
  }, [capsQuery.data, threadModel]);

  const capOptions: SelectProps["options"] = useMemo(() => {
    if (!maxTokens) return [];
    const options: SelectProps["options"] = [];

    const maxLabel = `${formatContextSize(maxTokens)} (max)`;
    options.push({
      value: String(maxTokens),
      textValue: maxLabel,
      children: maxLabel,
    });

    for (const fixedValue of FIXED_OPTIONS) {
      if (fixedValue < maxTokens && fixedValue >= MIN_CONTEXT_CAP) {
        const isMin = fixedValue === MIN_CONTEXT_CAP;
        const label = isMin
          ? `${formatContextSize(fixedValue)} (min)`
          : formatContextSize(fixedValue);
        options.push({
          value: String(fixedValue),
          textValue: label,
          children: label,
        });
      }
    }

    return options;
  }, [maxTokens]);

  const handleCapChange = useCallback(
    (value: string) => {
      dispatch(
        setContextTokensCap({
          chatId,
          value: parseInt(value, 10),
        }),
      );
    },
    [dispatch, chatId],
  );

  // Compute a safe default value that's guaranteed to exist in options
  const safeDefaultValue = useMemo(() => {
    if (!maxTokens || capOptions.length === 0) return undefined;

    // Get all valid option values as numbers
    const optionValues = capOptions
      .filter(
        (opt): opt is SelectProps["options"][number] & { value: string } =>
          typeof opt === "object" && "value" in opt,
      )
      .map((opt) => Number(opt.value));

    const desiredValue = contextCap ?? maxTokens;

    // If desired value exists in options, use it
    if (optionValues.includes(desiredValue)) {
      return String(desiredValue);
    }

    // Otherwise fall back to maxTokens (always the first option)
    return String(maxTokens);
  }, [capOptions, contextCap, maxTokens]);

  // Show skeleton while loading caps
  if (capsQuery.isLoading || capsQuery.isFetching) {
    return <Skeleton width="80px" height="24px" />;
  }

  if (!maxTokens || capOptions.length === 0 || !safeDefaultValue) return null;

  // Use model + maxTokens as key to force remount when either changes
  const selectKey = `${threadModel}-${maxTokens}`;

  return (
    <Select
      key={selectKey}
      title="Context cap"
      options={capOptions}
      defaultValue={safeDefaultValue}
      onChange={handleCapChange}
    />
  );
};
