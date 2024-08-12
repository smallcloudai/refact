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
import { useCallback, useEffect, useMemo } from "react";
import { selectConfig, setThemeMode } from "../features/Config/configSlice";
import { useMutationObserver } from "../hooks";
import { createAsyncThunk } from "@reduxjs/toolkit";

// export { type Config, setThemeMode } from "../features/Config/reducer";

// Use throughout your app instead of plain `useDispatch` and `useSelector`
export const useAppDispatch = useDispatch.withTypes<AppDispatch>();
export const useAppSelector = useSelector.withTypes<RootState>();
export const createAppAsyncThunk = createAsyncThunk.withTypes<{
  state: RootState;
  dispatch: AppDispatch;
}>();

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

  const selectAll = useMemo(() => {
    return (state: RootState) =>
      args.map((arg) => diffApi.endpoints.diffState.select(arg)(state));
  }, [args]);

  // Causes a wraning
  // TODO: use createSelector when messages are move into the state
  const all = useAppSelector(selectAll);

  const getByToolCallId = useCallback(
    (toolCallId: string) => {
      const item = all.find((d) => d.originalArgs?.toolCallId === toolCallId);
      return item;
    },
    [all],
  );

  const getByArg = (arg: DiffAppliedStateArgs) =>
    diffApi.endpoints.diffState.select(arg);

  return {
    allDiffRequest: all,
    getByToolCallId,
    getByArg,
  };
};

export const useConfig = () => useAppSelector(selectConfig);

export const useAppearance = () => {
  const config = useConfig();

  const appearance = config.themeProps.appearance;

  const handleChange = useCallback(() => {
    const maybeDark =
      document.body.classList.contains("vscode-dark") ||
      document.body.classList.contains("vscode-high-contrast");
    const maybeLight =
      document.body.classList.contains("vscode-light") ||
      document.body.classList.contains("vscode-high-contrast-light");

    if (maybeLight) {
      setThemeMode("light");
    } else if (maybeDark) {
      setThemeMode("dark");
    } else {
      setThemeMode("inherit");
    }
  }, []);

  useEffect(handleChange, [handleChange]);

  // TODO: remove this
  useMutationObserver(document.body, handleChange, {
    attributes: true,
    characterData: false,
    childList: false,
    subtree: false,
  });

  return {
    appearance,
    setApperance: setThemeMode,
  };
};
