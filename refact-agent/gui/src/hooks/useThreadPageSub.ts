import { useEffect } from "react";
import { useAppDispatch } from "./useAppDispatch";
import { useAppSelector } from "./useAppSelector";
import { threadsPageSub } from "../services/graphql/graphqlThunks";
import { selectActiveGroup } from "../features/Teams/teamsSlice";

export function useThreadPageSub() {
  const dispatch = useAppDispatch();

  const activeProject = useAppSelector(selectActiveGroup);

  useEffect(() => {
    if (activeProject === null) return;
    const thunk = dispatch(
      threadsPageSub({
        located_fgroup_id: activeProject.id,
        limit: 10,
      }),
    );

    return () => {
      thunk.abort("unmounted");
    };
  }, [activeProject, dispatch]);
}
