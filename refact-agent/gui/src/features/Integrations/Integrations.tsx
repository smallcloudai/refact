import React, { useCallback, useState } from "react";
import { Flex, Button } from "@radix-ui/themes";
import { ArrowLeftIcon } from "@radix-ui/react-icons";
import { ScrollArea } from "../../components/ScrollArea";
import { PageWrapper } from "../../components/PageWrapper";
import type { Config } from "../Config/configSlice";
import { useGetIntegrationsQuery } from "../../hooks/useGetIntegrationsDataQuery";
import { IntegrationsView } from "../../components/IntegrationsView";

export type IntegrationsProps = {
  onCloseIntegrations?: () => void;
  backFromIntegrations: () => void;
  handlePaddingShift: (state: boolean) => void;
  host: Config["host"];
  tabbed: Config["tabbed"];
};

export type LeftRightPadding =
  | {
      initial: string;
      xl: string;
      xs?: undefined;
      sm?: undefined;
      md?: undefined;
      lg?: undefined;
    }
  | {
      initial: string;
      xs: string;
      sm: string;
      md: string;
      lg: string;
      xl: string;
    };

export const Integrations: React.FC<IntegrationsProps> = ({
  onCloseIntegrations,
  backFromIntegrations,
  handlePaddingShift,
  host,
  tabbed,
}) => {
  const LeftRightPadding =
    host === "web"
      ? { initial: "5", xl: "9" }
      : {
          initial: "2",
          xs: "2",
          sm: "4",
          md: "8",
          lg: "8",
          xl: "9",
        };

  const { integrations } = useGetIntegrationsQuery();
  const [isInnerIntegrationSelected, setIsInnerIntegrationSelected] =
    useState<boolean>(false);

  const handleIfInnerIntegrationWasSet = useCallback(
    (state: boolean) => {
      setIsInnerIntegrationSelected(state);
      handlePaddingShift(state);
    },
    [handlePaddingShift],
  );

  return (
    <PageWrapper
      host={host}
      style={{
        padding: 0,
        marginTop: isInnerIntegrationSelected ? 50 : 0,
      }}
    >
      {!isInnerIntegrationSelected && (
        <>
          {host === "vscode" && !tabbed ? (
            <Flex gap="2" pb="3">
              <Button variant="surface" onClick={backFromIntegrations}>
                <ArrowLeftIcon width="16" height="16" />
                Back
              </Button>
            </Flex>
          ) : (
            <Button
              mr="auto"
              variant="outline"
              onClick={onCloseIntegrations}
              mb="4"
            >
              Back
            </Button>
          )}
        </>
      )}
      <ScrollArea scrollbars="vertical" fullHeight>
        <Flex
          direction="column"
          justify="between"
          flexGrow="1"
          mr={LeftRightPadding}
          pr={LeftRightPadding}
          style={{
            width: "inherit",
            height: "100%",
          }}
        >
          <IntegrationsView
            leftRightPadding={LeftRightPadding}
            handleIfInnerIntegrationWasSet={handleIfInnerIntegrationWasSet}
            integrationsMap={integrations.data}
            // integrationsIcons={icons.data}
            isLoading={integrations.isLoading}
            goBack={backFromIntegrations}
          />
        </Flex>
      </ScrollArea>
    </PageWrapper>
  );
};
