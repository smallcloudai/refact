import { useSelector } from "react-redux";
import type { RootState } from "../app/store";

export const useAppSelector = useSelector.withTypes<RootState>();
