import { useAppSelector } from "./useAppSelector";
import { getErrorMessage } from "../features/Errors/errorsSlice";
import { capsApi } from "../services/refact/caps";
import { useGetPing } from "./useGetPing";

export const useGetCapsQuery = () => {
  const error = useAppSelector(getErrorMessage);
  const pong = useGetPing();
  const skip = !!error || !pong.data;
  const caps = capsApi.useGetCapsQuery(undefined, {
    skip,
  });

  return caps;
};
