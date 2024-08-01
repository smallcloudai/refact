import { useDispatch, useSelector } from "react-redux";
import type { RootState, AppDispatch } from "./store";
import {
  statisticsApi,
  capsApi,
  promptsApi,
  toolsApi,
  commandsApi,
  CommandCompletionResponse,
  diffApi,
  DiffAppliedStateArgs,
} from "../services/refact";
import { useCallback, useEffect } from "react";

// Use throughout your app instead of plain `useDispatch` and `useSelector`
export const useAppDispatch = useDispatch.withTypes<AppDispatch>();
export const useAppSelector = useSelector.withTypes<RootState>();

export const { useGetStatisticDataQuery } = statisticsApi;
export const { useGetCapsQuery } = capsApi;
export const { useGetPromptsQuery } = promptsApi;

export const useGetToolsQuery = (hasCaps: boolean) => {
  return toolsApi.useGetToolsQuery(undefined, { skip: !hasCaps });
};

export const useGetCommandCompletionQuery = (
  query: string,
  cursor: number,
  hasCaps: boolean,
): CommandCompletionResponse => {
  const { data } = commandsApi.useGetCommandCompletionQuery(
    { query, cursor },
    { skip: !hasCaps },
  );

  if (!data) {
    return {
      completions: [],
      replace: [0, 0],
      is_cmd_executable: false,
    };
  }

  return data;
};

export const useGetCommandPreviewQuery = (query: string, hasCaps: boolean) => {
  const { data } = commandsApi.useGetCommandPreviewQuery(query, {
    skip: !hasCaps,
  });
  if (!data) return [];
  return data;
};

export const { useDiffApplyMutation, useDiffStateQuery } = diffApi;

export const useGetManyDiffState = (args: DiffAppliedStateArgs[]) => {
  const dispatch = useAppDispatch();

  useEffect(() => {
    const results = args.map((arg) =>
      dispatch(diffApi.endpoints.diffState.initiate(arg)),
    );
    return () => {
      results.forEach((result) => result.unsubscribe());
    };
  }, [args, dispatch]);

  args.map((d) => {
    const result = diffApi.endpoints.diffState.select(d);

    return result;
  });

  const all = useAppSelector((state) => {
    return args.map((arg) => {
      return diffApi.endpoints.diffState.select(arg)(state);
    });
  });

  const getByToolCallId = useCallback(
    (toolCallId: string) => {
      const item = all.find((d) => d.originalArgs?.toolCallId === toolCallId);
      return item;
    },
    [all],
  );

  return {
    allDiffRequest: all,
    getByToolCallId,
  };
};
