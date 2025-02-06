import { Badge, Card, Flex, Text } from "@radix-ui/themes";
import styles from "./IntegrationCard.module.css";
import {
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";
import { FC } from "react";
import classNames from "classnames";
import { useAppSelector } from "../../../hooks";
import { selectConfig } from "../../../features/Config/configSlice";
import { getIntegrationInfo } from "../../../utils/getIntegrationInfo";
import { formatIntegrationIconPath } from "../../../utils/formatIntegrationIconPath";

type IntegrationCardProps = {
  integration:
    | IntegrationWithIconRecord
    | NotConfiguredIntegrationWithIconRecord;
  handleIntegrationShowUp: (
    integration:
      | IntegrationWithIconRecord
      | NotConfiguredIntegrationWithIconRecord,
  ) => void;
  isNotConfigured?: boolean;
};

export const IntegrationCard: FC<IntegrationCardProps> = ({
  integration,
  handleIntegrationShowUp,
  isNotConfigured = false,
}) => {
  const config = useAppSelector(selectConfig);
  const port = config.lspPort;

  const iconPath = formatIntegrationIconPath(integration.icon_path);
  const integrationLogo = `http://127.0.0.1:${port}/v1${iconPath}`;

  const { displayName } = getIntegrationInfo(integration.integr_name);

  return (
    <Card
      className={classNames(styles.integrationCard, {
        [styles.integrationCardInline]: isNotConfigured,
      })}
      onClick={() => handleIntegrationShowUp(integration)}
    >
      <Flex
        gap="4"
        direction={isNotConfigured ? "column" : "row"}
        align={"center"}
      >
        <img
          src={integrationLogo}
          className={styles.integrationIcon}
          alt={integration.integr_name}
        />
        <Flex
          align="center"
          justify="between"
          gap={isNotConfigured ? "0" : "2"}
          width={isNotConfigured ? "auto" : "100%"}
        >
          <Text
            size="3"
            weight="medium"
            align={isNotConfigured ? "center" : "left"}
          >
            {displayName}
          </Text>
          {!isNotConfigured && (
            <Badge
              color={integration.on_your_laptop ? "jade" : "gray"}
              variant="soft"
              radius="medium"
            >
              {integration.on_your_laptop ? "On" : "Off"}
            </Badge>
          )}
        </Flex>
      </Flex>
    </Card>
  );
};
