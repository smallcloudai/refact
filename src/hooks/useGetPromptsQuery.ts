import { useAppSelector } from "./useAppSelector";
import { getErrorMessage } from "../features/Errors/errorsSlice";
import { promptsApi } from "../services/refact/prompts";

export const useGetPromptsQuery = () => {
  const error = useAppSelector(getErrorMessage);
  return promptsApi.useGetPromptsQuery(undefined, { skip: !!error });
};
