import {
  Button,
  Flex,
  Heading,
  HoverCard,
  Skeleton,
  Switch,
  Text,
} from "@radix-ui/themes";
import { AnimatePresence, motion } from "framer-motion";
import React, { useState } from "react";
import {
  ChevronLeftIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";

import { useGetToolGroupsQuery } from "../../../hooks";
import { ToolGroup as ToolGroupType } from "../../../services/refact";

import { ScrollArea } from "../../ScrollArea";
import { ToolGroup } from "./ToolGroup";

export const ToolGroups: React.FC = () => {
  const { data: toolsGroups, isLoading, isSuccess } = useGetToolGroupsQuery();
  const [selectedToolGroup, setSelectedToolGroup] =
    useState<ToolGroupType | null>(null);

  if (isLoading || !isSuccess) return <ToolGroupsSkeleton />;

  return (
    <Flex direction="column" gap="3" style={{ overflow: "hidden" }}>
      <Heading size="3" as="h3">
        Manage Tool Groups
      </Heading>
      <AnimatePresence mode="wait" initial={false}>
        {!selectedToolGroup ? (
          <motion.div
            key="group-list"
            initial={{ opacity: 0, x: -40 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -40 }}
            transition={{ duration: 0.25 }}
          >
            <ScrollArea
              scrollbars="vertical"
              type="auto"
              style={{
                maxHeight: "125px",
              }}
            >
              <Flex
                direction="column"
                gap="1"
                pr={toolsGroups.length < 4 ? "0" : "3"}
              >
                {toolsGroups.map((toolGroup) => (
                  <ToolGroup
                    key={toolGroup.name}
                    group={toolGroup}
                    setSelectedToolGroup={setSelectedToolGroup}
                  />
                ))}
              </Flex>
            </ScrollArea>
          </motion.div>
        ) : (
          <motion.div
            key="tools-list"
            initial={{ opacity: 0, x: 40 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 40 }}
            transition={{ duration: 0.25 }}
          >
            <Flex direction="column" gap="3">
              <Flex align="center" gap="3">
                <Button
                  variant="outline"
                  size="1"
                  onClick={() => setSelectedToolGroup(null)}
                  aria-label="Back"
                >
                  <ChevronLeftIcon />
                  Back
                </Button>
                <Heading as="h4" size="2">
                  {selectedToolGroup.name}
                </Heading>
              </Flex>
              <Button size="1" variant="outline" color="gray" mb="2">
                Unselect all
              </Button>
              <ScrollArea
                scrollbars="vertical"
                type="auto"
                style={{ maxHeight: "125px" }}
              >
                <Flex direction="column" gap="3" pr="4">
                  {selectedToolGroup.tools.map((tool) => (
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
                        <HoverCard.Root>
                          <HoverCard.Trigger>
                            <QuestionMarkCircledIcon
                              style={{ marginLeft: 4 }}
                            />
                          </HoverCard.Trigger>
                          <HoverCard.Content size="1">
                            <Text as="p" size="2">
                              {tool.spec.description}
                            </Text>
                          </HoverCard.Content>
                        </HoverCard.Root>
                      </Flex>
                      <Switch size="1" defaultChecked={tool.enabled} />
                    </Flex>
                  ))}
                </Flex>
              </ScrollArea>
            </Flex>
          </motion.div>
        )}
      </AnimatePresence>
    </Flex>
  );
};

const ToolGroupsSkeleton: React.FC = () => {
  return (
    <Flex direction="column" gap="3" style={{ overflow: "hidden" }}>
      <Skeleton loading={true}>
        <Heading size="3" as="h3">
          Manage Tool Groups
        </Heading>
      </Skeleton>
      <Flex direction="column" gap="1">
        {[1, 2].map((idx) => (
          <Flex key={idx} align="center" justify="between" gap="1">
            <Skeleton width="30px" height="25px" />
            <Skeleton width="100%" height="25px" />
            <Skeleton width="30px" height="25px" />
            <Skeleton width="30px" height="25px" />
          </Flex>
        ))}
      </Flex>
    </Flex>
  );
};
