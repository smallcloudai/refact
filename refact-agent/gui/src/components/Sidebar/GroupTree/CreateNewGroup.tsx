import { Cross2Icon, PlusIcon } from "@radix-ui/react-icons";
import {
  Button,
  Flex,
  Heading,
  HoverCard,
  IconButton,
  TextField,
} from "@radix-ui/themes";
import React, { useState } from "react";
import { TreeNodeData } from "./CustomTreeNode";
import { TreeApi } from "react-arborist";
import { v4 as uuidv4 } from "uuid";

export type CreateNewGroupProps<T> = {
  currentGroup: T;
  tree: TreeApi<T>;
  updateTree: (newTree: T[]) => void;
};

export const CreateNewGroup = <T extends TreeNodeData>({
  currentGroup,
  tree,
  updateTree,
}: CreateNewGroupProps<T>) => {
  const [isClicked, setIsClicked] = useState(false);
  const [groupName, setGroupName] = useState("");

  const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    setIsClicked((prev) => !prev);
  };

  const handleCreateGroup = () => {
    const updatedGroup = {
      ...currentGroup,
      children: currentGroup.children
        ? [...currentGroup.children, { id: uuidv4(), name: groupName }]
        : [{ id: uuidv4(), name: groupName }],
    };

    const updatedTree = updateNodeById(
      tree.props.data,
      currentGroup.id,
      () => updatedGroup,
    );

    updateTree(updatedTree);
    // TODO: send actual request to create a new group
    setIsClicked(false);
  };

  return (
    <Flex gap="2" flexShrink={"0"}>
      <HoverCard.Root open={isClicked}>
        <HoverCard.Trigger>
          <IconButton size="1" variant="soft" onClick={handleClick}>
            {isClicked ? <Cross2Icon /> : <PlusIcon />}
          </IconButton>
        </HoverCard.Trigger>
        <HoverCard.Content size="1" side="bottom" align="end">
          <Flex direction="column" gap="2">
            <Heading as="h6" size="2">
              Create new group
            </Heading>
            {/* TODO: when typing KeyA, group tree gets new empty node inserted and then keyboard works fine :/ */}
            <TextField.Root
              placeholder="Group name..."
              onClick={(e) => e.stopPropagation()}
              onKeyDown={(e) => e.stopPropagation()}
              value={groupName}
              onChange={(e) => setGroupName(e.target.value)}
            />
            <Button
              size="2"
              variant="soft"
              color="gray"
              onClick={handleCreateGroup}
            >
              Create
            </Button>
          </Flex>
        </HoverCard.Content>
      </HoverCard.Root>
    </Flex>
  );
};

function updateNodeById<T extends TreeNodeData>(
  nodes: readonly T[] | undefined,
  id: string,
  updater: (node: T) => T,
): T[] {
  if (!nodes) return [];
  return nodes.map((node) => {
    if (node.id === id) {
      return updater(node);
    }
    if (node.children) {
      return {
        ...node,
        children: updateNodeById(node.children as T[], id, updater),
      };
    }
    return node;
  });
}
