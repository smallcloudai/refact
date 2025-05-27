import { Button, Card, Checkbox, Flex, Heading, Text } from "@radix-ui/themes";
import { FlexusTreeNode } from "../GroupTree";
import React, { useCallback, useState } from "react";

import styles from "./ConfirmGroupSelection.module.css";
import { useMutation } from "urql";
import {
  CreateGroupDocument,
  CreateGroupMutation,
  CreateGroupMutationVariables,
} from "../../../../../generated/documents";
import { useAppDispatch, useAppSelector } from "../../../../hooks";
import { selectConfig } from "../../../../features/Config/configSlice";
import { setError } from "../../../../features/Errors/errorsSlice";

export type ConfirmGroupSelectionProps = {
  currentSelectedTeamsGroupNode: FlexusTreeNode;
  setCurrentSelectedTeamsGroupNode: (node: FlexusTreeNode | null) => void;
  onGroupSelectionConfirm: (node: FlexusTreeNode) => void;
};

export const ConfirmGroupSelection: React.FC<ConfirmGroupSelectionProps> = ({
  currentSelectedTeamsGroupNode,
  setCurrentSelectedTeamsGroupNode,
  onGroupSelectionConfirm,
}) => {
  const dispatch = useAppDispatch();
  const currentWorkspaceName =
    useAppSelector(selectConfig).currentWorkspaceName ?? "New Project";

  const [_, createGroup] = useMutation<
    CreateGroupMutation,
    CreateGroupMutationVariables
  >(CreateGroupDocument);

  const [createFolderChecked, setCreateFolderChecked] = useState(false);
  const handleConfirmClick = useCallback(async () => {
    if (createFolderChecked) {
      const result = await createGroup({
        fgroup_name: currentWorkspaceName,
        fgroup_parent_id: currentSelectedTeamsGroupNode.treenodeId,
      });

      if (result.error) {
        dispatch(setError(result.error.message));
        return;
      }

      const newGroup = result.data?.group_create;
      if (newGroup) {
        const newNode: FlexusTreeNode = {
          treenodeId: newGroup.fgroup_id,
          treenodeTitle: newGroup.fgroup_name,
          treenodeType: "group",
          treenodePath: `${currentSelectedTeamsGroupNode.treenodePath}/group:${newGroup.fgroup_id}`,
          treenode__DeleteMe: false,
          treenode__InsertedLater: false,
          treenodeChildren: [],
          treenodeExpanded: false,
        };
        setCurrentSelectedTeamsGroupNode(newNode);
        onGroupSelectionConfirm(newNode);
      }
    } else {
      onGroupSelectionConfirm(currentSelectedTeamsGroupNode);
      setCurrentSelectedTeamsGroupNode(null);
    }
  }, [
    dispatch,
    createFolderChecked,
    createGroup,
    currentSelectedTeamsGroupNode,
    setCurrentSelectedTeamsGroupNode,
    onGroupSelectionConfirm,
    currentWorkspaceName,
  ]);

  return (
    <Card size="3" mt="4" className={styles.modalCard}>
      <Flex direction="column" gap="4" align="start" width="100%">
        <Heading as="h4" size="5" mb="2">
          Do you want to attach your current workspace to the&nbsp;
          <span className={styles.groupName}>
            {currentSelectedTeamsGroupNode.treenodeTitle}
          </span>
          &nbsp;group?
        </Heading>
        <Text size="2" color="gray" mb="3">
          This will help you sync your workspace with the selected group in the
          cloud.
        </Text>
        <Flex align="center" gap="3" mb="4">
          <Checkbox
            id="create-folder-checkbox"
            checked={createFolderChecked}
            onCheckedChange={(checked: boolean) =>
              setCreateFolderChecked(checked)
            }
          />
          <label
            htmlFor="create-folder-checkbox"
            className={styles.checkboxLabel}
          >
            I want to create a folder <b>{currentWorkspaceName}</b> in current
            selected group
          </label>
        </Flex>
        <Flex align="center" gap="3" justify="end" width="100%">
          <Button
            size="2"
            onClick={() => setCurrentSelectedTeamsGroupNode(null)}
            color="gray"
            variant="soft"
          >
            Cancel
          </Button>
          <Button size="2" onClick={() => void handleConfirmClick()}>
            Confirm
          </Button>
        </Flex>
      </Flex>
    </Card>
  );
};
