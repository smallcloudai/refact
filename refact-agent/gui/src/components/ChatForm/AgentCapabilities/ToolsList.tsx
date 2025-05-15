import React from "react";
import { motion } from "framer-motion";
import {
  Button,
  Flex,
  Heading,
  HoverCard,
  Switch,
  Text,
} from "@radix-ui/themes";
import {
  ChevronLeftIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";

import { ScrollArea } from "../../ScrollArea";

import { ToolGroup } from "../../../services/refact";

export type ToolsListProps = {
  group: ToolGroup;
  tools: ToolGroup["tools"];
  onBack: () => void;
  onToggleAll: (group: ToolGroup) => void;
  onToggle: ({
    tool,
    parentGroup,
    togglingTo,
  }: {
    tool: ToolGroup["tools"][number];
    parentGroup: ToolGroup;
    togglingTo: boolean;
  }) => void;
  someEnabled: boolean;
};

export const ToolsList: React.FC<ToolsListProps> = ({
  group,
  tools,
  onToggle,
  onBack,
  onToggleAll,
  someEnabled,
}) => {
  return (
    <motion.div
      key="tools-list"
      initial={{ opacity: 0, x: 40 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: 40 }}
      transition={{ duration: 0.25 }}
    >
      <Flex direction="column" gap="3">
        <Flex align="center" gap="3">
          <Button variant="outline" size="1" onClick={onBack} aria-label="Back">
            <ChevronLeftIcon />
            Back
          </Button>
          <Heading as="h4" size="2">
            {group.name}
          </Heading>
        </Flex>
        <Button
          onClick={() => onToggleAll(group)}
          size="1"
          variant="outline"
          color="gray"
          mb="2"
        >
          {someEnabled ? "Unselect" : "Select"} all
        </Button>
        <ScrollArea
          scrollbars="vertical"
          type="auto"
          style={{ maxHeight: "125px" }}
        >
          <Flex direction="column" gap="3" pr="4">
            {tools.map((tool) => (
              <Flex
                key={tool.spec.name}
                align="center"
                gap="4"
                justify="between"
              >
                <Flex align="center" gap="2">
                  <Text as="p" size="2">
                    🔨 {tool.spec.display_name}
                  </Text>
                  {tool.spec.description.trim() !== "" && (
                    <HoverCard.Root>
                      <HoverCard.Trigger>
                        <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
                      </HoverCard.Trigger>
                      <HoverCard.Content size="1">
                        <Text as="p" size="2">
                          {tool.spec.description}
                        </Text>
                      </HoverCard.Content>
                    </HoverCard.Root>
                  )}
                </Flex>
                <Switch
                  size="1"
                  checked={tool.enabled}
                  onCheckedChange={(newState) =>
                    onToggle({
                      tool,
                      parentGroup: group,
                      togglingTo: newState,
                    })
                  }
                />
              </Flex>
            ))}
          </Flex>
        </ScrollArea>
      </Flex>
    </motion.div>
  );
};
