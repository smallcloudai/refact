import { diffApi, type DiffAppliedStateArgs } from "../services/refact/diffs";

export const useDiffStateQuery = (args: DiffAppliedStateArgs) => {
  return diffApi.useDiffStateQuery(args);
};
