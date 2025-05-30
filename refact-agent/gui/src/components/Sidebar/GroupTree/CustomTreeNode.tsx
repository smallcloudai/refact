import React, { useCallback } from "react";
import { Box, Flex, Text } from "@radix-ui/themes";
import { ChevronDownIcon, ChevronRightIcon } from "@radix-ui/react-icons";
import type { NodeRendererProps } from "react-arborist";
import { FolderIcon } from "./FolderIcon";

import styles from "./CustomTreeNode.module.css";
import { TeamsGroup } from "../../../services/smallcloud/types";
import { FlexusTreeNode } from "./GroupTree";

export type TeamsGroupTree = TeamsGroup & {
  children?: TeamsGroup[];
};

export const CustomTreeNode = <T extends FlexusTreeNode>({
  node,
  style,
  dragHandle,
}: NodeRendererProps<T> & { updateTree: (newTree: T[]) => void }) => {
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

  // Select the appropriate icon based on node type and state
  const getIcon = () => {
    if (isContainingChildren) {
      return node.isOpen ? (
        <ChevronDownIcon
          width={16}
          height={16}
          style={{
            color: node.isSelected ? "var(--accent-9)" : "var(--gray-10)",
            transition: "transform 0.2s ease, color 0.2s ease",
          }}
        />
      ) : (
        <ChevronRightIcon
          width={16}
          height={16}
          style={{
            color: node.isSelected ? "var(--accent-9)" : "var(--gray-10)",
            transition: "transform 0.2s ease, color 0.2s ease",
          }}
        />
      );
    }
    return <FolderIcon />;
  };

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
      {/* Icon container */}
      <Box
        onClick={isContainingChildren ? handleChevronClick : undefined}
        style={{
          display: "flex",
          alignItems: "center",
          marginRight: isContainingChildren ? 12 : 8,
          cursor: isContainingChildren ? "pointer" : "default",
          color: node.isSelected ? "var(--accent-9)" : "var(--gray-11)",
          flexShrink: 0,
        }}
      >
        {getIcon()}
      </Box>

      {isContainingChildren && (
        <FolderIcon width={16} height={16} open={node.isOpen} />
      )}

      <Text
        size="2"
        weight={"regular"}
        style={{
          flexGrow: 1,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
          color: "inherit",
          marginLeft: isContainingChildren ? 8 : 4,
          minWidth: 0, // This helps text truncation work properly
        }}
        title={node.data.treenodeTitle}
      >
        {node.data.treenodeTitle}
      </Text>
    </Flex>
  );
};
