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
import { AnimatePresence } from "framer-motion";

import { CustomTreeNode } from "./CustomTreeNode";
import { ConfirmGroupSelection } from "./ConfirmGroupSelection";
import { ScrollArea } from "../../ScrollArea";

import { useGroupTree } from "./useGroupTree";
import styles from "./GroupTree.module.css";

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
    handleSkipWorkspaceSelection,
    setGroupTreeData,
    onWorkspaceSelection,
    availableWorkspaces,
    treeHeight,
  } = useGroupTree();

  return (
    <Flex direction="column" gap="4" mt="4" width="100%">
      <Flex direction="column" gap="1">
        <Heading as="h2" size="4">
          Refact Teams Wizard
        </Heading>
        <Separator size="4" my="2" />
        <Heading as="h2" size="3">
          Account selection
        </Heading>
        <Text size="2" color="gray" mb="1">
          Refact is even better connected to the cloud, you can share knowledge
          database within your team.
        </Text>
        <Text size="2" color="gray" mb="1">
          Choose your team&apos;s account, or your personal account:
        </Text>
        <Select.Root
          onValueChange={onWorkspaceSelection}
          disabled={availableWorkspaces.length === 0}
          value={currentTeamsWorkspace?.ws_id}
        >
          <Select.Trigger placeholder="Please, choose team's account"></Select.Trigger>
          <Select.Content position="popper">
            {availableWorkspaces.map((workspace) => (
              <Select.Item value={workspace.ws_id} key={workspace.ws_id}>
                {workspace.root_group_name}
              </Select.Item>
            ))}
          </Select.Content>
        </Select.Root>
        {availableWorkspaces.length === 0 && (
          <Text size="2" mt="2">
            No accounts are currently associated with your team. Please contact
            your Team Workspace administrator to request access. For further
            assistance, please refer to the support or bug reporting channels.
          </Text>
        )}
        <Button
          onClick={handleSkipWorkspaceSelection}
          variant="outline"
          color="gray"
          mt="2"
        >
          Skip Selection
        </Button>
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
              Group selection
            </Heading>
            <Text size="2" color="gray">
              If you have a lot of projects, you can organize them into groups:
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
                <CustomTreeNode updateTree={setGroupTreeData} {...nodeProps} />
              )}
            </Tree>
          </ScrollArea>
          <AnimatePresence>
            {currentSelectedTeamsGroupNode !== null && (
              <ConfirmGroupSelection
                currentSelectedTeamsGroupNode={currentSelectedTeamsGroupNode}
                setCurrentSelectedTeamsGroupNode={
                  setCurrentSelectedTeamsGroupNode
                }
                onGroupSelectionConfirm={(group) =>
                  void onGroupSelectionConfirm(group)
                }
              />
            )}
          </AnimatePresence>
        </Flex>
      )}
    </Flex>
  );
};
