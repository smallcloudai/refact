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
import { TreeApi } from "react-arborist";
import { v4 as uuidv4 } from "uuid";
import { FlexusTreeNode } from "./GroupTree";

export type CreateNewGroupProps<T> = {
  currentGroup: T;
  tree: TreeApi<T>;
  updateTree: (newTree: T[]) => void;
};

export const CreateNewGroup = <T extends FlexusTreeNode>({
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

  const handleCreateGroup = (e: React.MouseEvent<HTMLButtonElement>) => {
    e.stopPropagation();
    const updatedGroup = {
      ...currentGroup,
      children: currentGroup.treenodeChildren
        ? [...currentGroup.treenodeChildren, { id: uuidv4(), name: groupName }]
        : [{ id: uuidv4(), name: groupName }],
    };

    const updatedTree = updateNodeByPath(
      tree.props.data,
      currentGroup.treenodePath,
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

function updateNodeByPath<T extends FlexusTreeNode>(
  nodes: readonly T[] | undefined,
  path: string,
  updater: (node: T) => T,
): T[] {
  if (!nodes) return [];
  return nodes.map((node) => {
    if (node.treenodePath === path) {
      return updater(node);
    }
    if (node.treenodeChildren) {
      return {
        ...node,
        treenodeChildren: updateNodeByPath(
          node.treenodeChildren as T[],
          path,
          updater,
        ),
      };
    }
    return node;
  });
}
