import { Button, Flex, Heading, IconButton } from "@radix-ui/themes";
import { useWindowDimensions } from "../../../hooks/useWindowDimensions.ts";
import type { FC } from "react";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import styles from "./IntegrationsHeader.module.css";
import { LeftRightPadding } from "../../../features/Integrations/Integrations.tsx";
import { toPascalCase } from "../../../utils/toPascalCase.ts";
import { useAppSelector } from "../../../hooks/index.ts";
import { selectConfig } from "../../../features/Config/configSlice.ts";

type IntegrationsHeaderProps = {
  handleFormReturn: () => void;
  integrationName: string;
  leftRightPadding: LeftRightPadding;
  icon: string;
  instantBackReturnment?: boolean;
  handleInstantReturn?: () => void;
};

export const IntegrationsHeader: FC<IntegrationsHeaderProps> = ({
  handleFormReturn,
  integrationName,
  leftRightPadding,
  icon,
  instantBackReturnment = false,
  handleInstantReturn,
}) => {
  const { width } = useWindowDimensions();
  const config = useAppSelector(selectConfig);

  const handleButtonClick = () => {
    if (instantBackReturnment && handleInstantReturn) {
      handleInstantReturn();
    } else {
      handleFormReturn();
    }
  };

  const isCmdline = integrationName.startsWith("cmdline");
  const isService = integrationName.startsWith("service");
  const isMCP = integrationName.startsWith("mcp");

  const getIntegrationDisplayName = () => {
    if (!integrationName.includes("TEMPLATE"))
      return toPascalCase(integrationName);
    if (isCmdline) return "Command Line Tool";
    if (isService) return "Service";
    if (isMCP) return "MCP Server";
  };

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
              {instantBackReturnment ? "Back to chat" : "Configurations"}
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
              {getIntegrationDisplayName()}
            </Heading>
          </Flex>
        </Flex>
      </Flex>
    </Flex>
  );
};
