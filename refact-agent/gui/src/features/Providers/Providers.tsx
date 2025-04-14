import React from "react";
import { Flex, Button } from "@radix-ui/themes";
import { ArrowLeftIcon } from "@radix-ui/react-icons";

import { ScrollArea } from "../../components/ScrollArea";
import { PageWrapper } from "../../components/PageWrapper";
import { Spinner } from "../../components/Spinner";
import { ProvidersView } from "./ProvidersView";
import { ProviderUpdateProvider } from "./ProviderUpdateContext";

import { useGetConfiguredProvidersQuery } from "../../hooks/useProvidersQuery";

import type { Config } from "../Config/configSlice";

export type ProvidersProps = {
  backFromProviders: () => void;
  host: Config["host"];
  tabbed: Config["tabbed"];
};
export const Providers: React.FC<ProvidersProps> = ({
  backFromProviders,
  host,
  tabbed,
}) => {
  const { data: configuredProvidersData, isSuccess } =
    useGetConfiguredProvidersQuery();

  if (!isSuccess) return <Spinner spinning />;
  return (
    <PageWrapper
      host={host}
      style={{
        padding: 0,
        marginTop: 0,
      }}
    >
      {host === "vscode" && !tabbed ? (
        <Flex gap="2" pb="3">
          <Button variant="surface" onClick={backFromProviders}>
            <ArrowLeftIcon width="16" height="16" />
            Back
          </Button>
        </Flex>
      ) : (
        <Button mr="auto" variant="outline" onClick={backFromProviders} mb="4">
          Back
        </Button>
      )}
      <ScrollArea scrollbars="vertical" fullHeight>
        <Flex
          direction="column"
          justify="between"
          flexGrow="1"
          style={{
            width: "inherit",
            height: "100%",
          }}
        >
          <ProviderUpdateProvider>
            <ProvidersView
              configuredProviders={configuredProvidersData.providers}
            />
          </ProviderUpdateProvider>
        </Flex>
      </ScrollArea>
    </PageWrapper>
  );
};
