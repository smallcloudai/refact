import { useAppSelector } from "./useAppSelector";
import { selectConfig } from "../features/Config/configSlice";

export const useConfig = () => useAppSelector(selectConfig);
