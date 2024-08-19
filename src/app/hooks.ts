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
  DiffOperationArgs,
  DiffAppliedStateArgs,
} from "../services/refact";
import { useCallback, useEffect } from "react";
import {
  selectConfig,
  selectLspPort,
  setThemeMode,
} from "../features/Config/configSlice";
import { useMutationObserver } from "../hooks/useMutationObserver";
import { createAsyncThunk, createSelector } from "@reduxjs/toolkit";
import { getErrorMessage } from "../features/Errors/errorsSlice";

// export { type Config, setThemeMode } from "../features/Config/reducer";

// Use throughout your app instead of plain `useDispatch` and `useSelector`

export const useAppDispatch = useDispatch.withTypes<AppDispatch>();
export const useAppSelector = useSelector.withTypes<RootState>();

type CreateAppAsyncThunk = ReturnType<
  typeof createAsyncThunk.withTypes<{
    state: RootState;
    dispatch: AppDispatch;
  }>
>;
// TODO: this cause a circular dependency issue :/
export const createAppAsyncThunk: CreateAppAsyncThunk =
  createAsyncThunk.withTypes<{
    state: RootState;
    dispatch: AppDispatch;
  }>();

// export const { useGetStatisticDataQuery } = statisticsApi;
export const useGetStatisticDataQuery = () => {
  const lspPort = useAppSelector(selectLspPort);
  return statisticsApi.useGetStatisticDataQuery({ port: lspPort });
};
// export const { useGetCapsQuery } = capsApi;
export const useGetCapsQuery = () => {
  const lspPort = useAppSelector(selectLspPort);
  const error = useAppSelector(getErrorMessage);
  return capsApi.useGetCapsQuery({ port: lspPort }, { skip: !!error });
};

// export const { useGetPromptsQuery } = promptsApi;

export const useGetPromptsQuery = () => {
  const error = useAppSelector(getErrorMessage);
  const lspPort = useAppSelector(selectLspPort);
  return promptsApi.useGetPromptsQuery({ port: lspPort }, { skip: !!error });
};

const selectCaps = (state: RootState) =>
  capsApi.endpoints.getCaps.select({ port: state.config.lspPort })(state);

const selectHasCaps = createSelector([selectCaps], (caps) => {
  if (!caps.data) return false;
  return true;
});

// const selectTools = (state: RootState) =>
//   toolsApi.endpoints.getTools.select({ port: state.config.lspPort })(state);

// const selectHasTools = createSelector([selectTools], (tools) => {
//   if (!tools.data) return false;
//   return tools.data.length > 0;
// });

export const useGetToolsQuery = () => {
  const lspPort = useAppSelector(selectLspPort);
  const hasCaps = useAppSelector(selectHasCaps);
  return toolsApi.useGetToolsQuery({ port: lspPort }, { skip: !hasCaps });
};

export const useGetCommandCompletionQuery = (
  query: string,
  cursor: number,
): CommandCompletionResponse => {
  const lspPort = useAppSelector(selectLspPort);
  const hasCaps = useAppSelector(selectHasCaps);
  const { data } = commandsApi.useGetCommandCompletionQuery(
    { query, cursor, port: lspPort },
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

export const useGetCommandPreviewQuery = (query: string) => {
  const hasCaps = useAppSelector(selectHasCaps);
  const port = useAppSelector(selectLspPort);
  const { data } = commandsApi.useGetCommandPreviewQuery(
    { query, port },
    {
      skip: !hasCaps,
    },
  );
  if (!data) return [];
  return data;
};

export const useDiffApplyMutation = () => {
  const port = useAppSelector(selectLspPort);
  const [submit, result] = diffApi.useDiffApplyMutation();

  const onSubmit = useCallback(
    (args: DiffOperationArgs) => {
      return submit({ port, ...args });
    },
    [port, submit],
  );

  return { onSubmit, result };
};

export const useDiffStateQuery = (args: DiffAppliedStateArgs) => {
  const port = useAppSelector(selectLspPort);
  return diffApi.useDiffStateQuery({ port, ...args });
};

export const useConfig = () => useAppSelector(selectConfig);

export const useAppearance = () => {
  const config = useConfig();
  const dispatch = useAppDispatch();

  const appearance = config.themeProps.appearance;

  const handleChange = useCallback(() => {
    const maybeDark =
      document.body.classList.contains("vscode-dark") ||
      document.body.classList.contains("vscode-high-contrast");
    const maybeLight =
      document.body.classList.contains("vscode-light") ||
      document.body.classList.contains("vscode-high-contrast-light");

    if (maybeLight) {
      dispatch(setThemeMode("light"));
    } else if (maybeDark) {
      dispatch(setThemeMode("dark"));
    } else {
      dispatch(setThemeMode("inherit"));
    }
  }, [dispatch]);

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
    setAppearance: setThemeMode,
  };
};
