import { useAppSelector } from "./useAppSelector";
import { selectBalance } from "../features/CoinBalance";

export function useCoinBallance() {
  return useAppSelector(selectBalance);
}
