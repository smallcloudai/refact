import { Box, Flex, Heading, Select, Text } from "@radix-ui/themes";
import React from "react";
import { Tree } from "react-arborist";
import { CustomTreeNode } from "./CustomTreeNode";

import styles from "./GroupTree.module.css";
import { ConfirmGroupSelection } from "./ConfirmGroupSelection";
import { useGroupTree } from "./useGroupTree";
import { ScrollArea } from "../../ScrollArea";

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
  const {
    treeParentRef,
    currentSelectedTeamsGroupNode,
    currentTeamsWorkspace,
    filteredGroupTreeData,
    onGroupSelect,
    onGroupSelectionConfirm,
    setCurrentSelectedTeamsGroupNode,
    setGroupTreeData,
    onWorkspaceSelection,
    availableWorkspaces,
    treeWidth,
    treeHeight,
  } = useGroupTree();

  return (
    <Flex direction="column" gap="6" mt="4" width="100%">
      <Flex direction="column" gap="1">
        <Heading as="h2" size="4">
          Choose workspace
        </Heading>
        <Text size="3" color="gray">
          Select a workspace associated to your team to continue.
        </Text>
        <Select.Root onValueChange={onWorkspaceSelection}>
          <Select.Trigger placeholder="Choose workspace"></Select.Trigger>
          <Select.Content position="popper">
            {availableWorkspaces.map((workspace) => (
              <Select.Item value={workspace.ws_id} key={workspace.ws_id}>
                {workspace.root_group_name}
              </Select.Item>
            ))}
          </Select.Content>
        </Select.Root>
      </Flex>
      {currentTeamsWorkspace && filteredGroupTreeData.length > 0 && (
        <Flex
          direction="column"
          gap="2"
          width="100%"
          height="100%"
          justify="between"
        >
          <Box width="100%">
            <Flex direction="column" gap="1" mb="4">
              <Heading as="h2" size="4">
                Choose desired group
              </Heading>
              <Text size="3" color="gray">
                Select a group to sync your knowledge with the cloud.
              </Text>
            </Flex>
            <ScrollArea
              ref={treeParentRef}
              scrollbars="vertical"
              style={{ maxHeight: 240 }}
            >
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
                  <CustomTreeNode
                    updateTree={setGroupTreeData}
                    {...nodeProps}
                  />
                )}
              </Tree>
            </ScrollArea>
          </Box>
          {/* TODO: make it wrapped around AnimatePresence from motion */}
          {currentSelectedTeamsGroupNode !== null && (
            <ConfirmGroupSelection
              currentSelectedTeamsGroupNode={currentSelectedTeamsGroupNode}
              setCurrentSelectedTeamsGroupNode={
                setCurrentSelectedTeamsGroupNode
              }
              onGroupSelectionConfirm={onGroupSelectionConfirm}
            />
          )}
        </Flex>
      )}
    </Flex>
  );
};
