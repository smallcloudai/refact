import React from "react";
import { Flex, RadioGroup, Text, HoverCard } from "@radix-ui/themes";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";

import { type ProjectLabelInfo } from "../../../utils/createProjectLabelsWithConflictMarkers";

export type IntegrationPathFieldProps = {
  configPath: string;
  projectPath: string;
  projectLabels: ProjectLabelInfo[];
  shouldBeFormatted: boolean;
  globalLabel?: string;
};

export const IntegrationPathField: React.FC<IntegrationPathFieldProps> = ({
  configPath,
  projectPath,
  projectLabels,
  shouldBeFormatted,
  globalLabel = "Global, available for all projects",
}) => {
  if (!shouldBeFormatted) {
    return (
      <Flex gap="2">
        <RadioGroup.Item value={configPath} /> {globalLabel}
      </Flex>
    );
  }

  const projectInfo = projectLabels.find((info) => info.path === projectPath);

  if (!projectInfo) {
    // Fallback to showing the full path if no project info found
    return (
      <Flex gap="2">
        <RadioGroup.Item value={configPath} /> {projectPath}
      </Flex>
    );
  }

  const content = (
    <Flex gap="2">
      <RadioGroup.Item value={configPath} /> {projectInfo.label}
    </Flex>
  );

  if (projectInfo.hasConflict) {
    return (
      <Flex align="center" gap="2">
        {content}
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon />
          </HoverCard.Trigger>
          <HoverCard.Content side="right" align="center" size="1">
            <Text size="1" as="p">
              Full project path:{" "}
            </Text>
            <Text size="1" as="span" color="gray">
              {projectInfo.fullPath}
            </Text>
          </HoverCard.Content>
        </HoverCard.Root>
      </Flex>
    );
  }

  return content;
};
