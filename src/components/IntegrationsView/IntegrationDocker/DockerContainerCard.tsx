import {
  Badge,
  Card,
  Code,
  DataList,
  DropdownMenu,
  Flex,
  Heading,
  IconButton,
} from "@radix-ui/themes";
import {
  DockerActionPayload,
  DockerContainer,
  SmartLink as SmartLinkType,
} from "../../../services/refact";
import { useState, type FC } from "react";
import { CopyIcon, DotsVerticalIcon } from "@radix-ui/react-icons";
import { toPascalCase } from "../../../utils/toPascalCase";
import { Markdown } from "../../Markdown";
import { Link } from "../../Link";
import { fallbackCopying } from "../../../utils/fallbackCopying";
import { TruncateRight } from "../../Text/TruncateRight";
import * as Collapsible from "@radix-ui/react-collapsible";
import { Reveal } from "../../Reveal";
import { Chevron } from "../../Collapsible";
import { SmartLink } from "../../SmartLink";

type DockerContainerCardProps = {
  containerSmartlinks?: SmartLinkType[];
  integrationData: {
    integrationName: string;
    integrationPath: string;
    integrationProject: string;
  };
  container: DockerContainer;
  currentContainerAction: DockerActionPayload | null;
  isActionInProgress: boolean;
  handleDockerContainerActionClick: (
    payload: DockerActionPayload,
  ) => Promise<void>;
};

const DOCKER_ACTIONS: (Omit<DockerActionPayload, "container"> & {
  label: string;
  loadingLabel: string;
})[] = [
  {
    label: "Start container",
    loadingLabel: "Starting...",
    action: "start",
  },
  {
    label: "Stop container",
    loadingLabel: "Stopping...",
    action: "stop",
  },
  {
    label: "Kill container",
    loadingLabel: "Killing...",
    action: "kill",
  },
  {
    label: "Remove container",
    loadingLabel: "Removing...",
    action: "remove",
  },
];

export const DockerContainerCard: FC<DockerContainerCardProps> = ({
  containerSmartlinks,
  integrationData,
  container,
  currentContainerAction,
  isActionInProgress,
  handleDockerContainerActionClick,
}) => {
  const [detailsOpen, setDetailsOpen] = useState(false);

  // needed to handle disabled state of buttons accordingly to the status of docker container
  const isDockerActionButtonDisabled = (
    el: DockerContainer,
    action: string,
  ) => {
    return (
      (isActionInProgress && currentContainerAction?.container === el.name) ||
      (el.status === "running" && action === "start") ||
      (el.status === "exited" && (action === "stop" || action === "kill"))
    );
  };

  const handleClickOnAction = ({
    container,
    action,
  }: {
    container: string;
    action: DockerActionPayload["action"];
  }) => {
    void handleDockerContainerActionClick({
      container,
      action,
    });
  };

  const formattedMarkdown = (text: string) => {
    return "```" + text + "```";
  };

  return (
    <Collapsible.Root open={detailsOpen}>
      <Card key={container.id}>
        <Flex direction="column" gap="4">
          <Flex direction="column" gap="4">
            <Collapsible.Trigger asChild>
              <Flex justify="between" gap="3">
                <Flex
                  gap="3"
                  align="center"
                  onClick={() => setDetailsOpen((prev) => !prev)}
                  style={{
                    cursor: "pointer",
                  }}
                >
                  <Heading as="h6" size="3" title={container.name}>
                    <Markdown>{formattedMarkdown(container.name)}</Markdown>
                  </Heading>
                  <Badge
                    color={container.status !== "running" ? "gray" : "jade"}
                    variant="soft"
                    radius="large"
                  >
                    {toPascalCase(container.status)}
                  </Badge>
                  <Chevron open={!detailsOpen} isUpDownChevron />
                </Flex>
                <Flex gap="3" align="center">
                  {/* actions for containers */}
                  <DropdownMenu.Root>
                    <DropdownMenu.Trigger>
                      <IconButton variant="outline" color="gray" size="1">
                        <DotsVerticalIcon />
                      </IconButton>
                    </DropdownMenu.Trigger>
                    <DropdownMenu.Content size="1" side="bottom" align="end">
                      {DOCKER_ACTIONS.map((dockerActionButton) => {
                        const action = dockerActionButton.action;
                        const label =
                          currentContainerAction?.action ===
                            dockerActionButton.action &&
                          currentContainerAction.container === container.name
                            ? dockerActionButton.loadingLabel
                            : dockerActionButton.label;

                        return (
                          <DropdownMenu.Item
                            key={dockerActionButton.label}
                            disabled={isDockerActionButtonDisabled(
                              container,
                              action,
                            )}
                            onClick={() => {
                              if (
                                isDockerActionButtonDisabled(container, action)
                              ) {
                                return;
                              }
                              handleClickOnAction({
                                container: container.name,
                                action: dockerActionButton.action,
                              });
                            }}
                            color={
                              dockerActionButton.action !== "start"
                                ? "red"
                                : undefined
                            }
                          >
                            {label}
                          </DropdownMenu.Item>
                        );
                      })}
                      {containerSmartlinks && (
                        <>
                          <DropdownMenu.Separator />
                          {containerSmartlinks.map((link, index) => (
                            <SmartLink
                              key={index}
                              smartlink={link}
                              integrationName={integrationData.integrationName}
                              integrationPath={integrationData.integrationPath}
                              integrationProject={
                                integrationData.integrationProject
                              }
                              isDockerSmartlink
                            />
                          ))}
                        </>
                      )}
                    </DropdownMenu.Content>
                  </DropdownMenu.Root>
                </Flex>
              </Flex>
            </Collapsible.Trigger>
          </Flex>
          <Collapsible.Content
            style={{
              marginTop: "1.15rem",
            }}
          >
            <Flex direction="column" gap="4">
              <Reveal defaultOpen={Object.values(container.env).length < 9}>
                <Flex direction="column" gap="2" align="start">
                  <Heading as="h5" size="4">
                    Environment variables
                  </Heading>
                  <DataList.Root size="1">
                    {Object.values(container.env).map((value) => {
                      const [variableName, variableValue] = value.split("=");
                      if (variableValue.startsWith("http")) {
                        return (
                          <DataList.Item key={variableName}>
                            <DataList.Label>{variableName}: </DataList.Label>
                            <DataList.Value>
                              <TruncateRight
                                size="1"
                                style={{
                                  width: "100%",
                                }}
                                title={variableValue}
                              >
                                <Link href={variableValue}>
                                  {variableValue}
                                </Link>
                              </TruncateRight>
                            </DataList.Value>
                          </DataList.Item>
                        );
                      }
                      return (
                        <DataList.Item key={variableName}>
                          <DataList.Label>{variableName}: </DataList.Label>
                          <DataList.Value>
                            <Flex align="center" gap="2" maxWidth="100%">
                              <TruncateRight
                                size="1"
                                style={{
                                  width: "100%",
                                }}
                                title={variableValue}
                              >
                                <Code
                                  variant="ghost"
                                  style={{
                                    width: "100%",
                                  }}
                                >
                                  {variableValue}
                                </Code>
                              </TruncateRight>
                              <IconButton
                                size="1"
                                title="Copy value"
                                color="gray"
                                variant="ghost"
                                onClick={() => fallbackCopying(variableValue)}
                              >
                                <CopyIcon />
                              </IconButton>
                            </Flex>
                          </DataList.Value>
                        </DataList.Item>
                      );
                    })}
                  </DataList.Root>
                </Flex>
              </Reveal>
            </Flex>
          </Collapsible.Content>
        </Flex>
      </Card>
    </Collapsible.Root>
  );
};
