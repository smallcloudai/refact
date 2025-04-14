import { FC } from "react";
import {
  Button,
  Popover,
  Box,
  Flex,
  Heading,
  Text,
  IconButton,
} from "@radix-ui/themes";
import classNames from "classnames";
import styles from "./DeletePopover.module.css";
import { TrashIcon } from "@radix-ui/react-icons";

export type DeletePopoverProps = {
  isDisabled: boolean;
  isDeleting: boolean;
  itemName: string;
  deleteBy: string;
  handleDelete: (deleteBy: string) => void;
};

export const DeletePopover: FC<DeletePopoverProps> = ({
  deleteBy,
  itemName,
  handleDelete,
  isDeleting,
  isDisabled,
}) => {
  return (
    <Popover.Root>
      <Popover.Trigger>
        <IconButton
          color="red"
          variant="outline"
          type="button"
          size="2"
          title={"Delete configuration data"}
          className={classNames({
            [styles.disabledButton]: isDeleting || isDisabled,
          })}
          disabled={isDeleting || isDisabled}
        >
          <TrashIcon width={20} height={20} />
        </IconButton>
      </Popover.Trigger>
      <Popover.Content width="360px">
        <Flex gap="3">
          <Box flexGrow="1">
            <Flex gap="4" justify="between" direction="column">
              <Flex direction="column" gap="2">
                <Heading as="h4" size="4">
                  Destructive action
                </Heading>
                <Text size="2">
                  Do you really want to delete {itemName}
                  &apos;s configuration data?
                </Text>
              </Flex>

              <Flex gap="3">
                <Popover.Close>
                  <Button
                    size="2"
                    variant="solid"
                    color="red"
                    onClick={() => handleDelete(deleteBy)}
                  >
                    Delete
                  </Button>
                </Popover.Close>
                <Popover.Close>
                  <Button size="2" variant="soft" color="gray">
                    Cancel
                  </Button>
                </Popover.Close>
              </Flex>
            </Flex>
          </Box>
        </Flex>
      </Popover.Content>
    </Popover.Root>
  );
};
