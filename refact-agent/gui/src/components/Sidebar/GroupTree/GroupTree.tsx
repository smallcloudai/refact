import {
  Button,
  Card,
  Flex,
  Heading,
  Link,
  Select,
  Text,
} from "@radix-ui/themes";
import React from "react";
import { Tree } from "react-arborist";

import { CustomTreeNode } from "./CustomTreeNode";
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
    handleSkipWorkspaceSelection,
    setGroupTreeData,
    onWorkspaceSelectChange,
    handleConfirmSelectionClick,
    handleCreateAccountClick,
    createFolderChecked,
    setCreateFolderChecked,
    availableWorkspaces,
    treeHeight,
  } = useGroupTree();

  return (
    <Flex direction="column" gap="4" mt="4" width="100%">
      <Flex direction="column" gap="1">
        <Heading as="h1" size="4" mb="1">
          Welcome to Refact.ai
        </Heading>
        <Text size="2" color="gray" mb="1">
          Refact.ai Agent autonomously completes your software engineering tasks
          end to end — and now comes with memory, turning your individual or
          team experience into a continuously evolving knowledge base.
        </Text>
        <Heading as="h2" size="3" mt="4">
          Select an Account
        </Heading>
        <Text size="1" color="gray" mb="1">
          Use your individual account or ask admin for access to your
          team&apos;s shared account
        </Text>
        <Select.Root
          onValueChange={onWorkspaceSelectChange}
          value={currentTeamsWorkspace?.ws_id}
          disabled={availableWorkspaces.length === 0}
        >
          <Select.Trigger placeholder="Select an Account" />
          <Select.Content position="popper">
            {availableWorkspaces.map((workspace) => (
              <Select.Item value={workspace.ws_id} key={workspace.ws_id}>
                {workspace.root_group_name}
              </Select.Item>
            ))}
          </Select.Content>
        </Select.Root>
        {availableWorkspaces.length === 0 && (
          <>
            <Text size="1" mt="2">
              No accounts are currently associated with your team. Please
              contact your Team Workspace administrator to request access. For
              further assistance, please refer to the support or bug reporting
              channels. You can create a new account by clicking link below:
            </Text>
            <Link href="#" size="1" mt="1" onClick={handleCreateAccountClick}>
              Add new account
            </Link>
          </>
        )}
      </Flex>
      {currentTeamsWorkspace && filteredGroupTreeData.length > 0 && (
        <Card>
          <Flex
            px="2"
            py="2"
            direction="column"
            gap="2"
            width="100%"
            height="100%"
            justify="between"
            style={{ flex: 1 }}
          >
            <Flex direction="column" gap="1" mb="1">
              <Heading as="h2" size="3">
                Select a Group
              </Heading>
              <Text size="1" color="gray">
                If you have several projects, organize them into groups
              </Text>
            </Flex>
            <ScrollArea
              ref={treeParentRef}
              scrollbars="vertical"
              style={{ flex: 1, minHeight: "100%" }}
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
        </Card>
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
