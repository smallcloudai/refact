import { useAppSelector } from "./useAppSelector";
import { selectBalance } from "../features/CoinBalance/coinBalanceSlice";

export function useCoinBallance() {
  return useAppSelector(selectBalance);
}
