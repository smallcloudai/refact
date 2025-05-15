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
import React, { useCallback, useEffect, useMemo, useState } from "react";
import {
  ChevronLeftIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";

import { useAppDispatch, useGetToolGroupsQuery } from "../../../hooks";
import {
  Tool,
  ToolGroup as ToolGroupType,
  ToolGroupUpdate,
  toolsApi,
  ToolSpec,
} from "../../../services/refact";

import { ScrollArea } from "../../ScrollArea";
import { ToolGroup } from "./ToolGroup";
import { useUpdateToolGroupsMutation } from "../../../hooks/useUpdateToolGroupsMutation";
import { debugApp } from "../../../debugConfig";

export const ToolGroups: React.FC = () => {
  const dispatch = useAppDispatch();
  const { data: toolsGroups, isLoading, isSuccess } = useGetToolGroupsQuery();
  const { mutationTrigger: updateToolGroups } = useUpdateToolGroupsMutation();

  const [selectedToolGroup, setSelectedToolGroup] =
    useState<ToolGroupType | null>(null);
  const [selectedToolGroupTools, setSelectedToolGroupTools] = useState<
    Tool[] | null
  >(null);

  const someToolsEnabled = useMemo(() => {
    if (!selectedToolGroup) return false;
    return selectedToolGroup.tools.some((tool) => tool.enabled);
  }, [selectedToolGroup]);

  useEffect(() => {
    if (selectedToolGroup) {
      setSelectedToolGroupTools(selectedToolGroup.tools);
    }
  }, [selectedToolGroup]);

  const handleUpdateToolGroups = useCallback(
    ({
      updatedTools,
      updatedGroup,
    }: {
      updatedTools: { enabled: boolean; spec: ToolSpec }[];
      updatedGroup: ToolGroupType;
    }) => {
      const dataToSend: ToolGroupUpdate[] = updatedTools.map((tool) => ({
        enabled: tool.enabled,
        source: tool.spec.source,
        name: tool.spec.name,
      }));
      debugApp(`[DEBUG]: updating data: `, dataToSend);

      updateToolGroups(dataToSend)
        .then((result) => {
          debugApp(`[DEBUG]: result: `, result);
          if (result.data) {
            // it means, individual tool update
            debugApp(`[DEBUG]: updating individual tool: `, updatedTools[0]);
            if (selectedToolGroupTools && updatedTools.length === 1) {
              setSelectedToolGroupTools((prev) => {
                const tool = updatedTools[0];
                return prev
                  ? prev.map((t) => {
                      if (t.spec.name === tool.spec.name) {
                        return { ...t, enabled: tool.enabled };
                      }
                      return t;
                    })
                  : selectedToolGroupTools;
              });
              return;
            }
            setSelectedToolGroup((prev) => {
              debugApp(
                "[DEBUG]: Previous group: ",
                prev,
                "new group: ",
                updatedGroup,
              );
              return updatedGroup;
            });
          }
        })
        .catch(alert);
    },
    [updateToolGroups, setSelectedToolGroupTools, selectedToolGroupTools],
  );

  const handleToggleToolsInToolGroup = useCallback(
    (toolGroup: ToolGroupType) => {
      const updatedTools = toolGroup.tools.map((tool) => ({
        ...tool,
        enabled: someToolsEnabled ? false : true,
      }));

      const updatedGroup = { ...toolGroup, tools: updatedTools };

      handleUpdateToolGroups({
        updatedTools,
        updatedGroup,
      });
    },
    [handleUpdateToolGroups, someToolsEnabled],
  );

  const handleResetSelectedGroup = useCallback(() => {
    dispatch(toolsApi.util.invalidateTags(["TOOL_GROUPS"]));
    setSelectedToolGroup(null);
  }, [dispatch]);

  const handleToggleSpecificToolFromToolGroup = useCallback(
    ({
      tool,
      parentGroup,
      togglingTo,
    }: {
      tool: ToolGroupType["tools"][number];
      parentGroup: ToolGroupType;
      togglingTo: boolean;
    }) => {
      const updatedTools: Tool[] = [
        {
          enabled: togglingTo,
          spec: tool.spec,
        },
      ];

      const updatedGroup = {
        ...parentGroup,
        tools: parentGroup.tools.map((t) => {
          if (t.spec.name === tool.spec.name) {
            return { ...tool };
          }

          return { ...t };
        }),
      };

      handleUpdateToolGroups({
        updatedTools,
        updatedGroup,
      });
    },
    [handleUpdateToolGroups],
  );

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
                  onClick={handleResetSelectedGroup}
                  aria-label="Back"
                >
                  <ChevronLeftIcon />
                  Back
                </Button>
                <Heading as="h4" size="2">
                  {selectedToolGroup.name}
                </Heading>
              </Flex>
              <Button
                onClick={() => handleToggleToolsInToolGroup(selectedToolGroup)}
                size="1"
                variant="outline"
                color="gray"
                mb="2"
              >
                {someToolsEnabled ? "Unselect" : "Select"} all
              </Button>
              <ScrollArea
                scrollbars="vertical"
                type="auto"
                style={{ maxHeight: "125px" }}
              >
                <Flex direction="column" gap="3" pr="4">
                  {selectedToolGroupTools?.map((tool) => (
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
                        )}
                      </Flex>
                      <Switch
                        size="1"
                        checked={tool.enabled}
                        onCheckedChange={(newState) =>
                          handleToggleSpecificToolFromToolGroup({
                            tool,
                            parentGroup: selectedToolGroup,
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
        )}
      </AnimatePresence>
    </Flex>
  );
};

const ToolGroupsSkeleton: React.FC = () => {
  return (
    <Flex direction="column" gap="3" style={{ overflow: "hidden" }}>
      <Heading size="3" as="h3">
        Manage Tool Groups
      </Heading>
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
