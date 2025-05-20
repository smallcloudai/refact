import React, { useCallback, useEffect, useRef, useState } from "react";
import { Box, Flex, Heading, Text } from "@radix-ui/themes";
import { ChatHistory, type ChatHistoryProps } from "../ChatHistory";
import { Spinner } from "@radix-ui/themes";
import { useAppSelector, useAppDispatch, useResizeObserver } from "../../hooks";
import {
  ChatHistoryItem,
  deleteChatById,
} from "../../features/History/historySlice";
import { push } from "../../features/Pages/pagesSlice";
import { restoreChat } from "../../features/Chat/Thread";
import { FeatureMenu } from "../../features/Config/FeatureMenu";
import {
  resetActiveGroup,
  selectActiveGroup,
  setActiveGroup,
} from "../../features/Teams";
import { NodeApi, Tree } from "react-arborist";
import { CustomTreeNode, TreeNodeData } from "./CustomTreeNode";
import styles from "./TreeStyles.module.css";
import { teamsApi } from "../../services/refact";

export type SidebarProps = {
  takingNotes: boolean;
  className?: string;
  style?: React.CSSProperties;
} & Omit<
  ChatHistoryProps,
  | "history"
  | "onDeleteHistoryItem"
  | "onCreateNewChat"
  | "onHistoryItemClick"
  | "currentChatId"
>;

const groupTreeData = [
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

export const Sidebar: React.FC<SidebarProps> = ({ takingNotes, style }) => {
  // TODO: these can be lowered.
  const dispatch = useAppDispatch();
  const [setActiveGroupIdTrigger] = teamsApi.useSetActiveGroupIdMutation();
  const maybeSelectedTeamsGroup = useAppSelector(selectActiveGroup);
  const history = useAppSelector((app) => app.history, {
    // TODO: selector issue here
    devModeChecks: { stabilityCheck: "never" },
  });

  const onDeleteHistoryItem = useCallback(
    (id: string) => dispatch(deleteChatById(id)),
    [dispatch],
  );

  const onHistoryItemClick = useCallback(
    (thread: ChatHistoryItem) => {
      dispatch(restoreChat(thread));
      dispatch(push({ name: "chat" }));
    },
    [dispatch],
  );

  const onGroupSelect = useCallback(
    (nodes: NodeApi<TreeNodeData>[]) => {
      if (nodes.length === 0) return;
      const group = nodes[0].data;
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

  return (
    <Flex style={style}>
      <FeatureMenu />
      <Flex mt="4">
        <Box position="absolute" ml="5" mt="2">
          <Spinner loading={takingNotes} title="taking notes" />
        </Box>
      </Flex>

      {maybeSelectedTeamsGroup ? (
        <ChatHistory
          history={history}
          onHistoryItemClick={onHistoryItemClick}
          onDeleteHistoryItem={onDeleteHistoryItem}
        />
      ) : (
        <Flex direction="column" gap="4" mt="4" width="100%">
          <Flex direction="column" gap="1">
            <Heading as="h2" size="4">
              Choose desired group
            </Heading>
            <Text size="3" color="gray">
              Select a group to sync your knowledge with the cloud.
            </Text>
          </Flex>
          <Box ref={treeParentRef} height="100%" width="100%">
            <Tree
              initialData={groupTreeData}
              rowHeight={40}
              height={treeHeight}
              width={treeWidth}
              indent={28}
              onSelect={onGroupSelect}
              openByDefault={false}
              className={styles.sidebarTree}
              disableDrag
              disableMultiSelection
              disableEdit
              disableDrop
            >
              {CustomTreeNode}
            </Tree>
          </Box>
        </Flex>
      )}
    </Flex>
  );
};
