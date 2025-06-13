import { Button, Flex, Heading, IconButton } from "@radix-ui/themes";
import { useWindowDimensions } from "../../../hooks/useWindowDimensions.ts";
import type { FC } from "react";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import styles from "./IntegrationsHeader.module.css";
import { LeftRightPadding } from "../../../features/Integrations/Integrations.tsx";
import { useAppSelector } from "../../../hooks/index.ts";
import { selectConfig } from "../../../features/Config/configSlice.ts";
import { getIntegrationInfo } from "../../../utils/getIntegrationInfo";

type IntegrationsHeaderProps = {
  handleFormReturn: () => void;
  integrationName: string;
  leftRightPadding: LeftRightPadding;
  icon: string;
  instantBackReturn?: boolean;
  handleInstantReturn?: () => void;
};

export const IntegrationsHeader: FC<IntegrationsHeaderProps> = ({
  handleFormReturn,
  integrationName,
  leftRightPadding,
  icon,
  instantBackReturn = false,
  handleInstantReturn,
}) => {
  const { width } = useWindowDimensions();
  const config = useAppSelector(selectConfig);

  const handleButtonClick = () => {
    if (instantBackReturn && handleInstantReturn) {
      handleInstantReturn();
    } else {
      handleFormReturn();
    }
  };

  const { displayName } = getIntegrationInfo(integrationName);

  return (
    <Flex
      className={styles.IntegrationsHeader}
      px={leftRightPadding}
      pt={config.host === "web" ? "5" : "2"}
    >
      <Flex
        align="center"
        justify="between"
        width="100%"
        px={config.host === "web" ? leftRightPadding : undefined}
      >
        <Flex
          gap={{
            initial: "3",
            xs: "5",
          }}
          align="center"
        >
          {width > 500 ? (
            <Button size="2" variant="surface" onClick={handleButtonClick}>
              <ArrowLeftIcon width="16" height="16" />
              {instantBackReturn ? "Back to chat" : "Configurations"}
            </Button>
          ) : (
            <IconButton size="2" variant="surface" onClick={handleButtonClick}>
              <ArrowLeftIcon width="16" height="16" />
            </IconButton>
          )}
          <Flex
            gap={{
              initial: "2",
              xs: "3",
            }}
            align="center"
          >
            <img
              src={icon}
              className={styles.IntegrationsHeaderIcon}
              alt={integrationName}
            />
            <Heading as="h5" size="3">
              {displayName}
            </Heading>
          </Flex>
        </Flex>
      </Flex>
    </Flex>
  );
};
