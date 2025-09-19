import { useMemo } from "react";
import { selectActiveWorkspace } from "../features/Teams/teamsSlice";
import { useAppSelector } from "./useAppSelector";
import { useBasicStuffQuery } from "./useBasicStuffQuery";

export function useCoinBallance() {
  const user = useBasicStuffQuery();

  const activeWorkspace = useAppSelector(selectActiveWorkspace);
  const balance = useMemo(() => {
    const maybeWorkspaceWithCoins =
      user.data?.query_basic_stuff.workspaces.find(
        (w) => w.ws_id === activeWorkspace?.ws_id,
      );

    if (!maybeWorkspaceWithCoins) return null;

    return {
      have_coins_enough: maybeWorkspaceWithCoins.have_coins_enough,
      have_coins_exactly: maybeWorkspaceWithCoins.have_coins_exactly as number,
    };
  }, [user.data, activeWorkspace?.ws_id]);

  return balance;
}
