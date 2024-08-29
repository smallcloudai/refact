import { useAppSelector } from "../app/hooks";
import { selectConfig } from "../features/Config/configSlice";

export const useConfig = () => useAppSelector(selectConfig);
