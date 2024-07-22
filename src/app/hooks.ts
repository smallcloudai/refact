import { useDispatch, useSelector } from "react-redux";
import type { RootState, AppDispatch } from "./store";
import { statisticsApi } from "../services/refact/statistics";
import { FIMAction } from "../events";

// Use throughout your app instead of plain `useDispatch` and `useSelector`
export const useAppDispatch = useDispatch.withTypes<AppDispatch & FIMAction>();
export const useAppSelector = useSelector.withTypes<RootState>();

export const { useGetStatisticDataQuery } = statisticsApi;
