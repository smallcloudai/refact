import { Box, Button, Card, Flex, Heading, Text } from "@radix-ui/themes";
import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { NodeApi, Tree } from "react-arborist";
import { CustomTreeNode, type TeamsGroupTree } from "./CustomTreeNode";
import { setActiveGroup, resetActiveGroup } from "../../../features/Teams";
import { isDetailMessage, teamsApi } from "../../../services/refact";
import {
  useAppDispatch,
  useEventsBusForIDE,
  useResizeObserver,
} from "../../../hooks";

import styles from "./TreeStyles.module.css";
import { TeamsGroup } from "../../../services/smallcloud/types";
import { setError } from "../../../features/Errors/errorsSlice";
import { useSmartSubscription } from "../../../hooks/useSmartSubscription";

import {
  NavTreeSubsDocument,
  type NavTreeSubsSubscription,
} from "../../../../generated/documents";

export interface FlexusTreeNode {
  treenodePath: string;
  treenodeId: string;
  treenodeTitle: string;
  treenodeType: string;
  treenode__DeleteMe: boolean;
  treenode__InsertedLater: boolean;
  treenodeChildren?: FlexusTreeNode[];
  treenodeExpanded: boolean;
}

const ws_id = "solarsystem"; // TODO: how do we get proper ws_id?

export const GroupTree: React.FC = () => {
  const [groupTreeData, setGroupTreeData] = useState<FlexusTreeNode[]>([]);

  const filterNodesByNodeType = useCallback(
    (nodes: FlexusTreeNode[], type: string): FlexusTreeNode[] => {
      return nodes
        .filter((node) => node.treenodeType === type)
        .map((node) => {
          const children = node.treenodeChildren
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
        // Helper to recursively update the tree
        const updateTree = (
          list: FlexusTreeNode[],
          parts: string[],
          curPath: string,
        ): FlexusTreeNode[] => {
          if (parts.length === 0) return list;

          const [part, ...restParts] = parts;
          const nextPath = curPath ? `${curPath}/${part}` : part;

          let node = list.find((n) => n.treenodePath === nextPath);

          if (!node) {
            node = {
              treenodeId: id,
              treenodePath: nextPath,
              treenodeTitle: part,
              treenodeType: part.split(":")[0],
              treenode__DeleteMe: false,
              treenode__InsertedLater: false,
              treenodeChildren: [],
              treenodeExpanded: true,
            };
            // Insert new node immutably
            list = [...list, node];
          } else {
            // Copy node for immutability
            node = { ...node };
            list = list.map((n) => {
              if (n.treenodePath === nextPath) {
                // Update the node immutably
                const updatedNode = { ...n, treenode__DeleteMe: false };
                if (nextPath === path) {
                  updatedNode.treenodeTitle = title;
                  updatedNode.treenodeType = type;
                }
                updatedNode.treenodeChildren = updateTree(
                  n.treenodeChildren ? n.treenodeChildren : [],
                  restParts,
                  nextPath,
                );
                return updatedNode;
              }
              return n;
            });
          }

          node.treenode__DeleteMe = false;
          if (nextPath === path) {
            node.treenodeTitle = title;
            node.treenodeType = type;
          }

          node.treenodeChildren = updateTree(
            node.treenodeChildren ? node.treenodeChildren : [],
            restParts,
            nextPath,
          );

          return list;
        };

        const parts = path.split("/");
        return updateTree(prevTree, parts, "");
      });
    },
    [setGroupTreeData],
  );

  const handleEveryTreeUpdate = useCallback(
    (data: NavTreeSubsSubscription | undefined) => {
      const u = data?.tree_subscription;
      if (!u) return;

      switch (u.treeupd_action) {
        // case 'TREE_REBUILD_START':
        //   markForDelete(theNavTreeRoot.value);
        //   break;
        case "TREE_UPDATE":
          touchNode(
            u.treeupd_path,
            u.treeupd_title,
            u.treeupd_type,
            u.treeupd_id,
          );
          break;
        // case 'TREE_REBUILD_FINISHED':
        //   setTimeout(() => {
        //     pruneInPlace(theNavTreeRoot.value);
        //     initialLoad.value = false;
        //   }, 500);
        //   setTimeout(() => {
        //     cleanupInsertedLater(theNavTreeRoot.value);
        //   }, 3000);
        //   break;
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
  const [currentSelectedTeamsGroup, setCurrentSelectedTeamsGroup] =
    useState<TeamsGroup | null>(null);

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
    setCurrentSelectedTeamsGroup({
      id: groupNode.treenodeId, // INCORRECT LOGIC!
      name: groupNode.treenodeTitle,
    });
  }, []);

  const onGroupSelectionConfirm = useCallback(
    (group: TeamsGroupTree) => {
      const newGroup = {
        id: group.id,
        name: group.name,
      };

      setActiveTeamsGroupInIDE(newGroup);
      void setActiveGroupIdTrigger({
        group_id: group.id,
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
          selection={currentSelectedTeamsGroup?.id.toString()}
          disableDrag
          disableMultiSelection
          disableEdit
          disableDrop
          idAccessor={"treenodeId"}
          childrenAccessor={"treenodeChildren"}
        >
          {(nodeProps) => (
            <CustomTreeNode updateTree={setGroupTreeData} {...nodeProps} />
          )}
        </Tree>
        {/* TODO: make it wrapped around AnimatePresence from motion */}
        {currentSelectedTeamsGroup !== null && (
          <Card size="2" mt="2">
            <Flex direction="column" gap="2" align="start">
              <Flex
                direction={{ initial: "column", xs: "row" }}
                align={{ initial: "start", xs: "center" }}
                justify="between"
                gap="2"
              >
                <Heading as="h4" size="3">
                  Confirm group selection:
                </Heading>
                <Text as="span" size="3">
                  {currentSelectedTeamsGroup.name}
                </Text>
              </Flex>
              <Flex align="center" gap="2">
                <Button
                  size="2"
                  onClick={() => setCurrentSelectedTeamsGroup(null)}
                  color="gray"
                  variant="soft"
                >
                  Cancel
                </Button>
                <Button
                  size="2"
                  onClick={() => {
                    setCurrentSelectedTeamsGroup(null);
                    onGroupSelectionConfirm({
                      ...currentSelectedTeamsGroup,
                      id: currentSelectedTeamsGroup.id,
                    });
                  }}
                  // disabled={currentSelectedTeamsGroup === null}
                >
                  Confirm
                </Button>
              </Flex>
            </Flex>
          </Card>
        )}
      </Box>
    </Flex>
  );
};
