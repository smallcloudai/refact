import { ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { Badge, Button, Flex, Text } from "@radix-ui/themes";
import { FC } from "react";
import { Integration } from "../../../services/refact";
import { useAppSelector } from "../../../hooks";
import { useEventsBusForIDE } from "../../../hooks/useEventBusForIDE";
import { selectConfig } from "../../../features/Config/configSlice";
import { DeletePopover } from "../../DeletePopover";

type ErrorStateProps = {
  integration: Integration;
  onDelete: (path: string) => void;
  isApplying: boolean;
  isDeletingIntegration: boolean;
};

export const ErrorState: FC<ErrorStateProps> = ({
  onDelete,
  isApplying,
  isDeletingIntegration,
  integration,
}) => {
  const config = useAppSelector(selectConfig);
  const { openFile } = useEventsBusForIDE();

  const { integr_name } = integration;
  const { error_msg, integr_config_path, error_line } =
    integration.error_log[0];

  return (
    <Flex width="100%" direction="column" align="start" gap="4">
      <Text size="2" color="gray">
        Whoops, this integration has a syntax error in the config file. You can
        fix this by editing the config file.
      </Text>
      <Badge size="2" color="red">
        <ExclamationTriangleIcon /> {error_msg}
      </Badge>
      <Flex align="center" gap="2">
        {config.host !== "web" && (
          <Button
            variant="outline"
            color="gray"
            title={`Open ${integr_name}.yaml configuration file in your IDE`}
            onClick={() =>
              openFile({
                file_path: integr_config_path,
                line: error_line === 0 ? 1 : error_line,
              })
            }
          >
            Open {integr_name}.yaml
          </Button>
        )}
        <DeletePopover
          itemName={integr_name}
          deleteBy={integr_config_path}
          isDisabled={isApplying}
          isDeleting={isDeletingIntegration}
          handleDelete={onDelete}
        />
      </Flex>
    </Flex>
  );
};
