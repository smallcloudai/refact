import { Badge, Card, Flex, Text } from "@radix-ui/themes";
import { toPascalCase } from "../../utils/toPascalCase";
import styles from "./IntegrationCard.module.css";
import {
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../services/refact";
import { FC } from "react";
import classNames from "classnames";
import { iconMap } from "./icons/iconMap";
import { useAppSelector } from "../../hooks";
import { selectThemeMode } from "../../features/Config/configSlice";

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

const INTEGRATIONS_WITH_TERMINAL_ICON = ["cmdline", "service"];

export const IntegrationCard: FC<IntegrationCardProps> = ({
  integration,
  handleIntegrationShowUp,
  isNotConfigured = false,
}) => {
  const theme = useAppSelector(selectThemeMode);
  const icons = iconMap(
    theme ? (theme === "inherit" ? "light" : theme) : "light",
  );

  const integrationLogo = INTEGRATIONS_WITH_TERMINAL_ICON.includes(
    integration.integr_name.split("_")[0],
  )
    ? icons.cmdline
    : icons[integration.integr_name];

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
            {integration.integr_name.includes("TEMPLATE")
              ? integration.integr_name.startsWith("cmdline")
                ? "Command-line Tool"
                : "Command-line Service"
              : toPascalCase(integration.integr_name)}
          </Text>
          {!isNotConfigured && (
            <Badge
              color={
                // TODO: get it back later integration.on_your_laptop || integration.when_isolated
                integration.on_your_laptop ? "jade" : "gray"
              }
              variant="soft"
              radius="medium"
            >
              {/* TODO: get it back later {integration.on_your_laptop || integration.when_isolated */}
              {integration.on_your_laptop ? "On" : "Off"}
            </Badge>
          )}
        </Flex>
      </Flex>
    </Card>
  );
};
