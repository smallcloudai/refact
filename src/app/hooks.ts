import { useDispatch, useSelector } from "react-redux";
import type { RootState, AppDispatch } from "./store";
import {
  diffApi,
  DiffOperationArgs,
  DiffAppliedStateArgs,
} from "../services/refact";
import { useCallback } from "react";
import { selectConfig, selectLspPort } from "../features/Config/configSlice";

// export { type Config, setThemeMode } from "../features/Config/reducer";

// Use throughout your app instead of plain `useDispatch` and `useSelector`

export const useAppDispatch = useDispatch.withTypes<AppDispatch>();
export const useAppSelector = useSelector.withTypes<RootState>();

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
