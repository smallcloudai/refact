import { useAppSelector } from "./useAppSelector";
import { getErrorMessage } from "../features/Errors/errorsSlice";
import { promptsApi } from "../services/refact/prompts";
import { useGetPing } from "./useGetPing";

export const useGetPromptsQuery = () => {
  const error = useAppSelector(getErrorMessage);
  const ping = useGetPing();
  const skip = !!error || !ping.data;

  return promptsApi.useGetPromptsQuery(undefined, { skip });
};
