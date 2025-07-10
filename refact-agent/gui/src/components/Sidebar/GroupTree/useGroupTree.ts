import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { FlexusTreeNode } from "../../../features/Groups";

import {
  useAppDispatch,
  useAppSelector,
  useBasicStuffQuery,
  useEventsBusForIDE,
  useOpenUrl,
  useResizeObserver,
} from "../../../hooks";
import { isDetailMessage, teamsApi } from "../../../services/refact";
import { NodeApi } from "react-arborist";
import {
  resetActiveGroup,
  resetActiveWorkspace,
  selectActiveWorkspace,
  setActiveGroup,
  setActiveWorkspace,
  setSkippedWorkspaceSelection,
} from "../../../features/Teams";
import { setError } from "../../../features/Errors/errorsSlice";
import { selectConfig } from "../../../features/Config/configSlice";

import {
  graphqlQueriesAndMutations,
  workspaceTreeSubscriptionThunk,
} from "../../../services/graphql";
import {
  cleanupWorkspaceInsertedLater,
  pruneWorkspaceNodes,
  selectWorkspaceState,
} from "../../../features/Groups";

export function useGroupTree() {
  const dispatch = useAppDispatch();
  // const [groupTreeData, setGroupTreeData] = useState<FlexusTreeNode[]>([]);
  const [createFolderChecked, setCreateFolderChecked] = useState(false);

  const currentTeamsWorkspace = useAppSelector(selectActiveWorkspace);
  const workspaceState = useAppSelector(selectWorkspaceState);
  const groupTreeData = useMemo(() => {
    return workspaceState.data;
  }, [workspaceState.data]);
  const openUrl = useOpenUrl();

  useEffect(() => {
    if (!currentTeamsWorkspace?.ws_id) return;

    const action = workspaceTreeSubscriptionThunk({
      ws_id: currentTeamsWorkspace.ws_id,
    });
    const thunk = dispatch(action);

    return () => thunk.abort();
  }, [currentTeamsWorkspace?.ws_id, dispatch]);

  const [createGroup] = graphqlQueriesAndMutations.useCreateGroupMutation();

  const teamsWorkspaces = useBasicStuffQuery();

  const filterNodesByNodeType = useCallback(
    (nodes: FlexusTreeNode[], type: string): FlexusTreeNode[] => {
      return nodes
        .filter((node) => node.treenodeType === type)
        .map((node) => {
          const children =
            node.treenodeChildren.length > 0
              ? filterNodesByNodeType(node.treenodeChildren, type)
              : [];
          return {
            ...node,
            treenodeChildren: children,
          };
        });
    },
    [],
  );

  const filteredGroupTreeData = useMemo(() => {
    return filterNodesByNodeType(groupTreeData, "group");
  }, [groupTreeData, filterNodesByNodeType]);

  useEffect(() => {
    if (workspaceState.finished) {
      setTimeout(() => {
        dispatch(pruneWorkspaceNodes());
      }, 500);
      setTimeout(() => {
        dispatch(cleanupWorkspaceInsertedLater());
      }, 3000);
    }
  }, [dispatch, workspaceState.finished]);

  const { setActiveTeamsGroupInIDE, setActiveTeamsWorkspaceInIDE } =
    useEventsBusForIDE();

  const [setActiveGroupIdTrigger] = teamsApi.useSetActiveGroupIdMutation();
  const [currentSelectedTeamsGroupNode, setCurrentSelectedTeamsGroupNode] =
    useState<FlexusTreeNode | null>(null);

  const treeParentRef = useRef<HTMLDivElement | null>(null);
  const [treeHeight, setTreeHeight] = useState<number>(
    treeParentRef.current?.clientHeight ?? 0,
  );

  const calculateAndSetSpace = useCallback(() => {
    if (!treeParentRef.current) {
      return;
    }
    setTreeHeight(treeParentRef.current.clientHeight);
    // NOTE: this is a hack to avoid the tree being with 0 width/height even when data appears
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [treeParentRef, filteredGroupTreeData]);

  useResizeObserver(treeParentRef.current, calculateAndSetSpace);

  useEffect(() => {
    calculateAndSetSpace();
  }, [calculateAndSetSpace]);

  const onGroupSelect = useCallback((nodes: NodeApi<FlexusTreeNode>[]) => {
    if (nodes.length === 0) return;
    const groupNode = nodes[0].data;
    setCurrentSelectedTeamsGroupNode(groupNode);
  }, []);

  const onGroupSelectionConfirm = useCallback(
    async (group: FlexusTreeNode) => {
      const newGroup = {
        id: group.treenodeId,
        name: group.treenodeTitle,
      };

      setActiveTeamsGroupInIDE(newGroup);
      try {
        const result = await setActiveGroupIdTrigger({
          group_id: group.treenodeId,
        });

        if (result.data) {
          dispatch(setActiveGroup(newGroup));
          return;
        } else {
          // TODO: rework error handling
          let errorMessage: string;
          if ("data" in result.error && isDetailMessage(result.error.data)) {
            errorMessage = result.error.data.detail;
          } else {
            errorMessage =
              "Error: Something went wrong while selecting a group. Try again.";
          }
          dispatch(setError(errorMessage));
        }
      } catch (e) {
        dispatch(resetActiveGroup());
      }
    },
    [dispatch, setActiveGroupIdTrigger, setActiveTeamsGroupInIDE],
  );

  const onWorkspaceSelectChange = useCallback(
    (value: string) => {
      const maybeWorkspace =
        teamsWorkspaces.data?.query_basic_stuff.workspaces.find(
          (w) => w.ws_id === value,
        );
      if (maybeWorkspace) {
        setActiveTeamsWorkspaceInIDE(maybeWorkspace);
        dispatch(setActiveWorkspace(maybeWorkspace));
        setCurrentSelectedTeamsGroupNode(null);
      }
    },
    [
      dispatch,
      setActiveTeamsWorkspaceInIDE,
      teamsWorkspaces.data?.query_basic_stuff.workspaces,
    ],
  );

  const handleCreateWorkspaceClick = useCallback(
    (event: React.MouseEvent<HTMLAnchorElement>) => {
      event.preventDefault();
      event.stopPropagation();
      openUrl("http://app.refact.ai/profile?action=create-workspace");
    },
    [openUrl],
  );

  const currentWorkspaceName =
    useAppSelector(selectConfig).currentWorkspaceName ?? "New Project";

  const isMatchingGroupNameWithWorkspace = useMemo(() => {
    return (
      currentSelectedTeamsGroupNode?.treenodeTitle === currentWorkspaceName
    );
  }, [currentSelectedTeamsGroupNode?.treenodeTitle, currentWorkspaceName]);

  const handleConfirmSelectionClick = useCallback(async () => {
    if (!currentSelectedTeamsGroupNode) return;
    if (createFolderChecked && !isMatchingGroupNameWithWorkspace) {
      const result = await createGroup({
        fgroup_name: currentWorkspaceName,
        fgroup_parent_id: currentSelectedTeamsGroupNode.treenodeId,
      });

      if (result.error) {
        dispatch(setError(JSON.stringify(result.error)));
        return;
      }

      const newGroup = result.data.group_create;

      const newNode: FlexusTreeNode = {
        treenodeId: newGroup.fgroup_id,
        treenodeTitle: newGroup.fgroup_name,
        treenodeType: "group",
        treenodePath: `${currentSelectedTeamsGroupNode.treenodePath}/group:${newGroup.fgroup_id}`,
        treenode__DeleteMe: false,
        treenode__InsertedLater: false,
        treenodeChildren: [],
        treenodeExpanded: false,
      };
      setCurrentSelectedTeamsGroupNode(newNode);
      void onGroupSelectionConfirm(newNode);
    } else {
      void onGroupSelectionConfirm(currentSelectedTeamsGroupNode);
      setCurrentSelectedTeamsGroupNode(null);
    }
  }, [
    dispatch,
    createGroup,
    currentSelectedTeamsGroupNode,
    setCurrentSelectedTeamsGroupNode,
    onGroupSelectionConfirm,
    currentWorkspaceName,
    createFolderChecked,
    isMatchingGroupNameWithWorkspace,
  ]);

  const handleSkipWorkspaceSelection = useCallback(() => {
    dispatch(setSkippedWorkspaceSelection(true));
    dispatch(resetActiveWorkspace());
  }, [dispatch]);

  const availableWorkspaces = useMemo(() => {
    if (teamsWorkspaces.data?.query_basic_stuff.workspaces) {
      return teamsWorkspaces.data.query_basic_stuff.workspaces;
    }
    return [];
  }, [teamsWorkspaces.data?.query_basic_stuff.workspaces]);

  useEffect(() => {
    if (availableWorkspaces.length === 1) {
      dispatch(setActiveWorkspace(availableWorkspaces[0]));
      setActiveTeamsWorkspaceInIDE(availableWorkspaces[0]);
    }
  }, [dispatch, setActiveTeamsWorkspaceInIDE, availableWorkspaces]);

  return {
    // Refs
    treeParentRef,
    // Data
    groupTreeData,
    filteredGroupTreeData,
    teamsWorkspaces,
    availableWorkspaces,
    // Current states
    currentTeamsWorkspace,
    currentSelectedTeamsGroupNode,
    createFolderChecked,
    // Dimensions
    treeHeight,
    // Actions
    onGroupSelect,
    onGroupSelectionConfirm,
    onWorkspaceSelectChange,
    // touchNode,
    handleSkipWorkspaceSelection,
    handleConfirmSelectionClick,
    handleCreateWorkspaceClick,
    // Setters
    // setGroupTreeData,
    setCurrentSelectedTeamsGroupNode,
    setCreateFolderChecked,
  };
}
