import { useAppSelector } from "../app/hooks";
import { getErrorMessage } from "../features/Errors/errorsSlice";
import { promptsApi } from "../services/refact/prompts";

export const useGetPromptsQuery = () => {
  const error = useAppSelector(getErrorMessage);
  return promptsApi.useGetPromptsQuery(undefined, { skip: !!error });
};
