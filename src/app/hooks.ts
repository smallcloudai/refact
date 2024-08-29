import { useDispatch, useSelector } from "react-redux";
import type { RootState, AppDispatch } from "./store";
import { selectConfig } from "../features/Config/configSlice";

// export { type Config, setThemeMode } from "../features/Config/reducer";

// Use throughout your app instead of plain `useDispatch` and `useSelector`

export const useAppDispatch = useDispatch.withTypes<AppDispatch>();
export const useAppSelector = useSelector.withTypes<RootState>();

export const useConfig = () => useAppSelector(selectConfig);
