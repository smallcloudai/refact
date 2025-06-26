import {
  Button,
  Flex,
  Heading,
  Select,
  Separator,
  Text,
} from "@radix-ui/themes";
import React from "react";
import { Tree } from "react-arborist";

import { CustomTreeNode } from "./CustomTreeNode";
import { ScrollArea } from "../../ScrollArea";
import { AnimatePresence } from "framer-motion";

import { useGroupTree } from "./useGroupTree";
import styles from "./GroupTree.module.css";
import { useOpenUrl } from "../../../hooks";

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

export const GroupTree: React.FC = () => {
  const openUrl = useOpenUrl();

  const {
    treeParentRef,
    currentSelectedTeamsGroupNode,
    currentTeamsWorkspace,
    filteredGroupTreeData,
    onGroupSelect,
    handleSkipWorkspaceSelection,
    setGroupTreeData,
    onWorkspaceSelection,
    handleConfirmSelectionClick,
    createFolderChecked,
    setCreateFolderChecked,
    availableWorkspaces,
    treeHeight,
  } = useGroupTree();

  return (
    <Flex direction="column" gap="4" mt="4" width="100%">
      <Flex direction="column" gap="1">
        <Heading as="h1" size="6" mb="1">
          Welcome to Refact.ai
        </Heading>
        <Text size="2" color="gray" mb="1">
          Refact.ai Agent autonomously completes your software engineering tasks
          end to end — and now comes with memory, turning your individual or
          team experience into a continuously evolving knowledge base.
        </Text>
        <Separator size="4" my="2" />
        <Heading as="h2" size="3" mb="1">
          Choose your Workspace
        </Heading>
        <Select.Root
          onValueChange={onWorkspaceSelection}
          // disabled={availableWorkspaces.length === 0}
          value={currentTeamsWorkspace?.ws_id}
        >
          <Select.Trigger></Select.Trigger>
          <Select.Content position="popper">
            {availableWorkspaces.map((workspace) => (
              <Select.Item value={workspace.ws_id} key={workspace.ws_id}>
                {workspace.root_group_name}
              </Select.Item>
            ))}
            {availableWorkspaces.length !== 0 && <Select.Separator />}
            <Select.Item
              value="add-new-workspace"
              onClickCapture={(e) => {
                e.preventDefault();
                e.stopPropagation();
                openUrl("https://app.refact.ai/profile");
              }}
            >
              Add new workspace
            </Select.Item>
          </Select.Content>
        </Select.Root>
        {availableWorkspaces.length === 0 && (
          <Text size="2" mt="2">
            No accounts are currently associated with your team. Please contact
            your Team Workspace administrator to request access. For further
            assistance, please refer to the support or bug reporting channels.
          </Text>
        )}
      </Flex>
      {currentTeamsWorkspace && filteredGroupTreeData.length > 0 && (
        <Flex
          direction="column"
          gap="2"
          width="100%"
          height="100%"
          justify="between"
          style={{ flex: 1, minHeight: 0 }}
        >
          <Flex direction="column" gap="1" mb="1">
            <Heading as="h2" size="3">
              Choose your Group
            </Heading>
            <Text size="2" color="gray">
              If you have several projects, organize them into groups
            </Text>
          </Flex>
          <ScrollArea
            ref={treeParentRef}
            scrollbars="vertical"
            style={{ flex: 1, minHeight: 0 }}
          >
            <Tree
              data={filteredGroupTreeData}
              rowHeight={40}
              height={treeHeight}
              width="100%"
              indent={28}
              onSelect={onGroupSelect}
              openByDefault={true}
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
                <CustomTreeNode
                  updateTree={setGroupTreeData}
                  createFolderChecked={createFolderChecked}
                  setCreateFolderChecked={setCreateFolderChecked}
                  {...nodeProps}
                />
              )}
            </Tree>
          </ScrollArea>
        </Flex>
      )}
      <Flex gap="2" justify="end">
        <Button
          onClick={handleSkipWorkspaceSelection}
          variant="outline"
          color="gray"
        >
          Skip
        </Button>
        <Button
          onClick={() => void handleConfirmSelectionClick()}
          variant="outline"
          disabled={currentSelectedTeamsGroupNode === null}
        >
          Confirm
        </Button>
      </Flex>
    </Flex>
  );
};
