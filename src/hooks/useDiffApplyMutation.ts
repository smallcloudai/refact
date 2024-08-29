import { useCallback } from "react";
import { diffApi, DiffOperationArgs } from "../services/refact/diffs";

export const useDiffApplyMutation = () => {
  const [submit, result] = diffApi.useDiffApplyMutation();

  const onSubmit = useCallback(
    (args: DiffOperationArgs) => {
      return submit(args);
    },
    [submit],
  );

  return { onSubmit, result };
};
