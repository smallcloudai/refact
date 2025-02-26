import { FC, MouseEventHandler } from "react";
import classNames from "classnames";

import { Badge, Card, Flex, Text } from "@radix-ui/themes";
import { useAppSelector } from "../../../hooks";
import { useUpdateIntegration } from "./useUpdateIntegration";

import {
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";

import { selectConfig } from "../../../features/Config/configSlice";
import { formatIntegrationIconPath } from "../../../utils/formatIntegrationIconPath";
import { getIntegrationInfo } from "../../../utils/getIntegrationInfo";

import styles from "./IntegrationCard.module.css";

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
  const state = useAppSelector((state) => state);

  if (!integration || !state) {
    return null;
  }

  const integrationLogo = formatIntegrationIconPath(integration.icon_path, state);

  const { displayName } = getIntegrationInfo(integration.integr_name);
  const {
    updateIntegrationAvailability,
    integrationAvailability,
    isUpdatingAvailability,
  } = useUpdateIntegration({ integration });

  const handleAvailabilityClick: MouseEventHandler<HTMLDivElement> = (
    event,
  ) => {
    if (isUpdatingAvailability) return;
    event.stopPropagation();
    void updateIntegrationAvailability();
  };

  const switches = [
    { label: "On", leftRadius: true },
    { label: "Off", rightRadius: true },
  ];

  return (
    <Card
      className={classNames(styles.integrationCard, {
        [styles.integrationCardInline]: isNotConfigured,
        [styles.disabledCard]: isUpdatingAvailability,
      })}
      onClick={() => {
        if (isUpdatingAvailability) return;
        handleIntegrationShowUp(integration);
      }}
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
          onError={(e) => {
            // On error, set a default image or hide the broken image icon
            e.currentTarget.style.display = 'none';
          }}
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
            <Flex
              className={classNames(styles.availabilitySwitch, {
                [styles.disabledAvailabilitySwitch]: isUpdatingAvailability,
              })}
              onClick={handleAvailabilityClick}
            >
              {switches.map(({ label, leftRadius }) => {
                const isOn = label === "On";
                const isActive =
                  isOn === (integrationAvailability?.on_your_laptop ?? false);

                return (
                  <Badge
                    key={label}
                    color={
                      isActive && !isUpdatingAvailability ? "jade" : "gray"
                    }
                    variant="soft"
                    radius="medium"
                    style={{
                      ...(leftRadius
                        ? {
                            borderTopRightRadius: 0,
                            borderBottomRightRadius: 0,
                          }
                        : {
                            borderTopLeftRadius: 0,
                            borderBottomLeftRadius: 0,
                          }),
                    }}
                  >
                    {label}
                  </Badge>
                );
              })}
            </Flex>
          )}
        </Flex>
      </Flex>
    </Card>
  );
};
