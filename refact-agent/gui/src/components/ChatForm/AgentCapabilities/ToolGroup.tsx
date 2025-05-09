import {
  ChevronRightIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import {
  Badge,
  BadgeProps,
  Box,
  Flex,
  Heading,
  HoverCard,
  Text,
  Tooltip,
} from "@radix-ui/themes";
import React, { useMemo } from "react";
import { ToolGroup as ToolGroupType } from "../../../services/refact";

import styles from "./ToolGroup.module.css";

export type ToolGroupProps = {
  group: ToolGroupType;
  setSelectedToolGroup: (group: ToolGroupType) => void;
};

export const ToolGroup: React.FC<ToolGroupProps> = ({
  group,
  setSelectedToolGroup,
}) => {
  const categoryBadge = useMemo(() => {
    const categoryMap: Record<
      string,
      { color: BadgeProps["color"]; tooltip: string }
    > = {
      builtin: { color: "red", tooltip: "Built-In Tools" },
      integration: { color: "yellow", tooltip: "Integrations Tools" },
      mcp: { color: "green", tooltip: "MCP Tools" },
    };

    const { color, tooltip } = categoryMap[group.category];
    const label = group.category.charAt(0).toUpperCase();

    return { label, color, tooltip };
  }, [group.category]);

  return (
    <Box
      key={group.name}
      onClick={() => setSelectedToolGroup(group)}
      py="1"
      pl="2"
      pr="1"
      className={styles.toolGroup}
    >
      <Heading as="h4" size="1">
        <Flex align="center" justify="between">
          <Flex as="span" align="center" gap="1">
            <Flex align="center" gap="2">
              <Tooltip content={categoryBadge.tooltip}>
                <Badge
                  size="1"
                  className={styles.categoryBadge}
                  color={categoryBadge.color}
                >
                  {categoryBadge.label}
                </Badge>
              </Tooltip>
              {group.name}
            </Flex>
            <HoverCard.Root>
              <HoverCard.Trigger>
                <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
              </HoverCard.Trigger>
              <HoverCard.Content size="1">
                <Text as="p" size="2">
                  {group.description}
                </Text>
              </HoverCard.Content>
            </HoverCard.Root>
          </Flex>
          <Flex align="center" gap="1">
            <Tooltip content="Indicates how many tools the group contains">
              <Badge color="indigo">{group.tools.length}</Badge>
            </Tooltip>
            <ChevronRightIcon />
          </Flex>
        </Flex>
      </Heading>
    </Box>
  );
};
