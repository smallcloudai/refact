import { useDispatch, useSelector } from "react-redux";
import type { RootState, AppDispatch } from "./store";
import { statisticsApi } from "../services/refact/statistics";
import { capsApi } from "../services/refact/caps";
import { promptsApi } from "../services/refact/prompts";
import { toolsApi } from "../services/refact/tools";

// Use throughout your app instead of plain `useDispatch` and `useSelector`
export const useAppDispatch = useDispatch.withTypes<AppDispatch>();
export const useAppSelector = useSelector.withTypes<RootState>();

export const { useGetStatisticDataQuery } = statisticsApi;
export const { useGetCapsQuery } = capsApi;
export const { useGetPromptsQuery } = promptsApi;
export const { useGetToolsQuery } = toolsApi;
