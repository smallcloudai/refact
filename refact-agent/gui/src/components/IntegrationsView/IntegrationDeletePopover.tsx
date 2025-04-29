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
import { FC } from "react";
import styles from "./IntegrationForm/IntegrationForm.module.css";
import { TrashIcon } from "@radix-ui/react-icons";

type IntegrationDeletePopoverProps = {
  isApplying: boolean;
  isDeletingIntegration: boolean;
  integrationName: string;
  integrationConfigPath: string;
  handleDeleteIntegration: (path: string, name: string) => void;
};

export const IntegrationDeletePopover: FC<IntegrationDeletePopoverProps> = ({
  isApplying,
  isDeletingIntegration,
  integrationName,
  integrationConfigPath,
  handleDeleteIntegration,
}) => {
  return (
    <Popover.Root>
      <Popover.Trigger>
        <IconButton
          color="red"
          variant="outline"
          type="button"
          size="2"
          title={"Delete configuration data of this integration"}
          className={classNames(
            {
              [styles.disabledButton]: isDeletingIntegration || isApplying,
            },
            // styles.button,
          )}
          disabled={isDeletingIntegration || isApplying}
        >
          {/* {isDeletingIntegration
            ? "Deleting configuration..."
            : "Delete configuration"} */}
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
                  Do you really want to delete {integrationName}
                  &apos;s configuration data?
                </Text>
              </Flex>

              <Flex gap="3">
                <Popover.Close>
                  <Button
                    size="2"
                    variant="solid"
                    color="red"
                    onClick={() =>
                      handleDeleteIntegration(
                        integrationConfigPath,
                        integrationName,
                      )
                    }
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
