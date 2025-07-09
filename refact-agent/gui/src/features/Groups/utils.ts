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

export const markForDelete = (nodes: FlexusTreeNode[]) => {
  const newNodes = [...nodes];
  newNodes.forEach((n) => {
    n.treenode__DeleteMe = true;
    n.treenodeChildren.length > 0 && markForDelete(n.treenodeChildren);
  });
  return newNodes;
};
export const cleanupInsertedLater = (nodes: FlexusTreeNode[]) => {
  const newNodes = [...nodes];
  newNodes.forEach((n) => {
    n.treenode__InsertedLater = false;
    n.treenodeChildren.length > 0 && cleanupInsertedLater(n.treenodeChildren);
  });
  return newNodes;
};

export const pruneNodes = (nodes: FlexusTreeNode[]): FlexusTreeNode[] => {
  const result: FlexusTreeNode[] = [];
  for (let i = nodes.length - 1; i >= 0; i--) {
    const n = nodes[i];
    if (n.treenode__DeleteMe) {
      // skip this node
      continue;
    }
    const prunedChildren =
      n.treenodeChildren.length > 0 ? pruneNodes(n.treenodeChildren) : [];

    result.push({
      ...n,
      treenodeChildren: prunedChildren,
    });
  }

  return result.reverse();
};

export const updateTree = (
  list: FlexusTreeNode[],
  parts: string[],
  curPath: string,
  id: string,
  path: string,
  title: string,
  type: string,
): FlexusTreeNode[] => {
  if (parts.length === 0) return list;

  const [part, ...restParts] = parts;
  const nextPath = curPath ? `${curPath}/${part}` : part;

  // Find node by path
  let node = list.find((n) => n.treenodePath === nextPath);

  if (!node) {
    // Create new node
    node = {
      treenodeId: id,
      treenodePath: nextPath,
      treenodeTitle: part,
      treenodeType: part.split(":")[0],
      treenode__DeleteMe: false,
      treenode__InsertedLater: false,
      treenodeChildren: [],
      treenodeExpanded: true,
    };
  }

  // Prepare updated node
  const updatedNode = { ...node, treenode__DeleteMe: false };

  if (nextPath === path) {
    updatedNode.treenodeTitle = title;
    updatedNode.treenodeType = type;
  }

  // Recursively update children
  updatedNode.treenodeChildren = updateTree(
    node.treenodeChildren,
    restParts,
    nextPath,
    id,
    path,
    title,
    type,
  );

  // Remove any node with the same path and add the updated node
  const filteredList = list.filter((n) => n.treenodePath !== nextPath);
  return [...filteredList, updatedNode];
};
