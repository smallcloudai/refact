import React, { useCallback, useMemo } from "react";
import { useAppDispatch, useAppSelector, useGetCapsQuery } from "../../hooks";
import {
  selectChatId,
  selectContextTokensCap,
  selectModel,
  selectThreadMaximumTokens,
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
  const threadMaxTokens = useAppSelector(selectThreadMaximumTokens);
  const threadModel = useAppSelector(selectModel);
  const capsQuery = useGetCapsQuery();

  // Use thread max tokens, or fall back to current model's n_ctx from caps
  const maxTokens = useMemo(() => {
    if (threadMaxTokens) return threadMaxTokens;
    if (!capsQuery.data) return undefined;

    // Try thread model first
    if (threadModel in capsQuery.data.chat_models) {
      return capsQuery.data.chat_models[threadModel].n_ctx;
    }

    // Fall back to default model
    const defaultModel = capsQuery.data.chat_default_model;
    if (defaultModel in capsQuery.data.chat_models) {
      return capsQuery.data.chat_models[defaultModel].n_ctx;
    }

    return undefined;
  }, [threadMaxTokens, capsQuery.data, threadModel]);

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

  // Show skeleton while loading caps
  if (capsQuery.isLoading || capsQuery.isFetching) {
    return <Skeleton width="80px" height="24px" />;
  }

  if (!maxTokens || capOptions.length === 0) return null;

  return (
    <Select
      title="Context cap"
      options={capOptions}
      value={String(contextCap ?? maxTokens)}
      onChange={handleCapChange}
    />
  );
};
