import { Box, Button, Card, Flex, Heading, Text } from "@radix-ui/themes";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { NodeApi, Tree } from "react-arborist";
import { CustomTreeNode, TreeNodeData } from "./CustomTreeNode";
import {
  setActiveGroup,
  resetActiveGroup,
  TeamsGroup,
} from "../../../features/Teams";
import { teamsApi } from "../../../services/refact";
import { useAppDispatch, useResizeObserver } from "../../../hooks";

import styles from "./TreeStyles.module.css";

const TEST_TREE_DATA = [
  { id: "1", name: "My Workspace 1" },
  {
    id: "2",
    name: "My Workspace 2",
    children: [
      { id: "3", name: "refact" },
      { id: "4", name: "refact-vscode" },
      { id: "5", name: "refact-scenarios" },
    ],
  },
];

export const GroupTree: React.FC = () => {
  const dispatch = useAppDispatch();
  const [groupTreeData, setGroupTreeData] =
    useState<TreeNodeData[]>(TEST_TREE_DATA);
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

  const onGroupSelect = useCallback((nodes: NodeApi<TreeNodeData>[]) => {
    if (nodes.length === 0) return;
    const group = nodes[0].data;
    setCurrentSelectedTeamsGroup({
      id: parseInt(group.id),
      name: group.name,
    });
  }, []);

  const onGroupSelectionConfirm = useCallback(
    (group: TreeNodeData) => {
      void setActiveGroupIdTrigger({
        group_id: parseInt(group.id),
      })
        .then((result) => {
          if (result.data) {
            // TODO: implement
            // setActiveWorkspaceInIDE(workspace);
            dispatch(
              setActiveGroup({
                id: parseInt(group.id),
                name: group.name,
              }),
            );
          }
        })
        .catch(() => {
          dispatch(resetActiveGroup());
        });
    },
    [dispatch, setActiveGroupIdTrigger],
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
          data={groupTreeData}
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
                      id: currentSelectedTeamsGroup.id.toString(),
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
