import { Box, Flex, Heading, Text } from "@radix-ui/themes";
import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { NodeApi, Tree } from "react-arborist";
import { CustomTreeNode } from "./CustomTreeNode";
import { setActiveGroup, resetActiveGroup } from "../../../features/Teams";
import { isDetailMessage, teamsApi } from "../../../services/refact";
import {
  useAppDispatch,
  useEventsBusForIDE,
  useResizeObserver,
} from "../../../hooks";

import { setError } from "../../../features/Errors/errorsSlice";
import { useSmartSubscription } from "../../../hooks/useSmartSubscription";

import {
  NavTreeSubsDocument,
  type NavTreeSubsSubscription,
} from "../../../../generated/documents";

import {
  cleanupInsertedLater,
  markForDelete,
  pruneNodes,
  updateTree,
} from "./utils";

import styles from "./GroupTree.module.css";
import { ConfirmGroupSelection } from "./ConfirmGroupSelection";

export interface FlexusTreeNode {
  treenodePath: string;
  treenodeId: string;
  treenodeTitle: string;
  treenodeType: string;
  treenode__DeleteMe: boolean;
  treenode__InsertedLater: boolean;
  treenodeChildren: FlexusTreeNode[];
  treenodeExpanded: boolean;
}

const ws_id = "31n8sWNX8Q"; // TODO: get proper ws_id from /v1/login workspaces
// const ws_id = "solarsystem";

export const GroupTree: React.FC = () => {
  const [groupTreeData, setGroupTreeData] = useState<FlexusTreeNode[]>([]);

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

  const touchNode = useCallback(
    (path: string, title: string, type: string, id: string) => {
      if (!path) return;
      setGroupTreeData((prevTree) => {
        const parts = path.split("/");
        return updateTree(prevTree, parts, "", id, path, title, type);
      });
    },
    [setGroupTreeData],
  );

  const handleEveryTreeUpdate = useCallback(
    (data: NavTreeSubsSubscription | undefined) => {
      const u = data?.tree_subscription;
      if (!u) return;
      switch (u.treeupd_action) {
        case "TREE_REBUILD_START":
          setGroupTreeData((prev) => markForDelete(prev));
          break;
        case "TREE_UPDATE":
          touchNode(
            u.treeupd_path,
            u.treeupd_title,
            u.treeupd_type,
            u.treeupd_id,
          );
          break;
        case "TREE_REBUILD_FINISHED":
          setTimeout(() => {
            setGroupTreeData((prev) => pruneNodes(prev));
          }, 500);
          setTimeout(() => {
            setGroupTreeData((prev) => cleanupInsertedLater(prev));
          }, 3000);
          break;
        default:
          // eslint-disable-next-line no-console
          console.warn("TREE SUBS:", u.treeupd_action);
      }
    },
    [touchNode],
  );

  useSmartSubscription<NavTreeSubsSubscription, { ws_id: string }>({
    query: NavTreeSubsDocument,
    variables: { ws_id },
    onUpdate: handleEveryTreeUpdate,
  });

  const dispatch = useAppDispatch();
  const { setActiveTeamsGroupInIDE } = useEventsBusForIDE();

  const [setActiveGroupIdTrigger] = teamsApi.useSetActiveGroupIdMutation();
  const [currentSelectedTeamsGroupNode, setCurrentSelectedTeamsGroupNode] =
    useState<FlexusTreeNode | null>(null);

  const treeParentRef = useRef<HTMLDivElement | null>(null);
  const [treeHeight, setTreeHeight] = useState<number>(
    treeParentRef.current?.clientHeight ?? 0,
  );
  const [treeWidth, setTreeWidth] = useState<number>(
    treeParentRef.current?.clientHeight ?? 0,
  );

  const calculateAndSetSpace = useCallback(() => {
    if (!treeParentRef.current) {
      return;
    }

    setTreeHeight(treeParentRef.current.clientHeight);
    setTreeWidth(treeParentRef.current.clientWidth);
  }, [treeParentRef]);

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
    (group: FlexusTreeNode) => {
      const newGroup = {
        id: group.treenodeId,
        name: group.treenodeTitle,
      };

      setActiveTeamsGroupInIDE(newGroup);
      void setActiveGroupIdTrigger({
        group_id: group.treenodeId,
      })
        .then((result) => {
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
        })
        .catch(() => {
          dispatch(resetActiveGroup());
        });
    },
    [dispatch, setActiveGroupIdTrigger, setActiveTeamsGroupInIDE],
  );

  return (
    <Flex direction="column" gap="4" mt="4" width="100%">
      <Flex direction="column" gap="1">
        <Heading as="h2" size="4">
          Choose desired group
        </Heading>
        <Text size="3" color="gray">
          Select a group to sync your knowledge with the cloud.
        </Text>
      </Flex>
      <Box ref={treeParentRef} height="240px" width="100%">
        <Tree
          data={filteredGroupTreeData}
          rowHeight={40}
          height={treeHeight}
          width={treeWidth}
          indent={28}
          onSelect={onGroupSelect}
          openByDefault={false}
          className={styles.sidebarTree}
          selection={currentSelectedTeamsGroupNode?.treenodePath}
          disableDrag
          disableMultiSelection
          disableEdit
          disableDrop
          idAccessor={"treenodePath"} // treenodePath seems to be more convenient for temporary tree nodes which later get removed
          childrenAccessor={"treenodeChildren"}
        >
          {(nodeProps) => (
            <CustomTreeNode updateTree={setGroupTreeData} {...nodeProps} />
          )}
        </Tree>
        {/* TODO: make it wrapped around AnimatePresence from motion */}
        {currentSelectedTeamsGroupNode !== null && (
          <ConfirmGroupSelection
            currentSelectedTeamsGroupNode={currentSelectedTeamsGroupNode}
            setCurrentSelectedTeamsGroupNode={setCurrentSelectedTeamsGroupNode}
            onGroupSelectionConfirm={onGroupSelectionConfirm}
          />
        )}
      </Box>
    </Flex>
  );
};
