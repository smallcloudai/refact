import { useAppSelector } from "../app/hooks";
import { getErrorMessage } from "../features/Errors/errorsSlice";
import { capsApi } from "../services/refact/caps";

export const useGetCapsQuery = () => {
  const error = useAppSelector(getErrorMessage);
  return capsApi.useGetCapsQuery(undefined, { skip: !!error });
};
