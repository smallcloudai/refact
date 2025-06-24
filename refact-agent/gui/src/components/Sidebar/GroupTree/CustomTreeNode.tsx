import React, { useCallback, useMemo } from "react";
import { Box, Checkbox, Flex, Text, Tooltip } from "@radix-ui/themes";
import { BookmarkFilledIcon, ChevronDownIcon } from "@radix-ui/react-icons";
import type { NodeRendererProps } from "react-arborist";
import { FolderIcon } from "./FolderIcon";

import styles from "./CustomTreeNode.module.css";
import { TeamsGroup } from "../../../services/smallcloud/types";
import { FlexusTreeNode } from "./GroupTree";
import { useAppSelector } from "../../../hooks";
import { selectConfig } from "../../../features/Config/configSlice";

export type TeamsGroupTree = TeamsGroup & {
  children?: TeamsGroup[];
};

export const CustomTreeNode = <T extends FlexusTreeNode>({
  node,
  style,
  createFolderChecked,
  setCreateFolderChecked,
  dragHandle,
}: NodeRendererProps<T> & {
  updateTree: (newTree: T[]) => void;
  createFolderChecked: boolean;
  setCreateFolderChecked: (state: boolean) => void;
}) => {
  const currentWorkspaceName =
    useAppSelector(selectConfig).currentWorkspaceName ?? "New Project";

  // Determine if this is a folder (has children)
  const isContainingChildren = node.data.treenodeChildren.length > 0;

  // Handle node click (for selection)
  const handleNodeClick = useCallback(() => {
    node.select();

    // If it doesn't contain children, also activate it
    if (!isContainingChildren) {
      node.activate();
    }
  }, [isContainingChildren, node]);

  // Handle chevron click (for expanding/collapsing)
  const handleChevronClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation(); // Prevent node selection
      if (isContainingChildren) {
        node.toggle();
      }
    },
    [isContainingChildren, node],
  );

  // Chevron icon for expandable nodes
  const getChevronIcon = () => {
    if (!isContainingChildren) return null;

    return (
      <ChevronDownIcon
        width={16}
        height={16}
        style={{
          transform: node.isOpen ? "rotate(0deg)" : "rotate(-90deg)",
          transition: "transform 0.2s ease, color 0.2s ease",
        }}
      />
    );
  };

  const isMatchingWorkspaceNameInIDE = useMemo(() => {
    return node.data.treenodeTitle === currentWorkspaceName;
  }, [node.data.treenodeTitle, currentWorkspaceName]);

  return (
    <Flex
      align="center"
      style={{
        ...style,
        backgroundColor: node.isSelected ? "var(--accent-3)" : "transparent",
      }}
      pr="2"
      onClick={handleNodeClick}
      ref={dragHandle}
      className={styles.treeNode}
    >
      <Box
        onClick={isContainingChildren ? handleChevronClick : undefined}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          width: 20,
          cursor: isContainingChildren ? "pointer" : "default",
          color: "var(--gray-11)",
          flexShrink: 0,
        }}
      >
        {getChevronIcon()}
      </Box>

      <Box
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          width: 20,
          flexShrink: 0,
        }}
        ml="2"
      >
        <FolderIcon
          width={16}
          height={16}
          open={isContainingChildren ? node.isOpen : false}
          style={{
            color: node.isSelected ? "var(--accent-9)" : "var(--gray-10)",
          }}
        />
      </Box>

      <Text
        size="2"
        weight="regular"
        ml="2"
        style={{
          flexGrow: 1,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          color: "inherit",
          minWidth: 0, // This helps text truncation work properly
        }}
        title={node.data.treenodeTitle}
      >
        {node.data.treenodeTitle}
      </Text>
      {node.isSelected && currentWorkspaceName !== node.data.treenodeTitle && (
        <Flex align="center" gap="3">
          <Text
            htmlFor="create-folder-checkbox"
            as="label"
            size="1"
            className={styles.checkboxLabel}
          >
            Create <Text weight="bold">{currentWorkspaceName}</Text> here
          </Text>
          <Checkbox
            id="create-folder-checkbox"
            checked={createFolderChecked}
            onCheckedChange={(checked: boolean) =>
              setCreateFolderChecked(checked)
            }
          />
        </Flex>
      )}
      {isMatchingWorkspaceNameInIDE && (
        <Tooltip
          content={`Current IDE workspace "${currentWorkspaceName}" may be a good match for this group`}
        >
          <BookmarkFilledIcon />
        </Tooltip>
      )}
    </Flex>
  );
};
