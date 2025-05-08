import {
  ChevronLeftIcon,
  ChevronRightIcon,
  MixerVerticalIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import {
  Badge,
  Button,
  Card,
  Flex,
  Heading,
  HoverCard,
  IconButton,
  Popover,
  Separator,
  Switch,
  Text,
  Tooltip,
} from "@radix-ui/themes";
import {
  AgentRollbackSwitch,
  ApplyPatchSwitch,
  FollowUpsSwitch,
} from "./ChatControls";
import { useAppSelector } from "../../hooks";
import {
  selectAreFollowUpsEnabled,
  selectAutomaticPatch,
  selectCheckpointsEnabled,
} from "../../features/Chat";
import { useEffect, useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { STUB_TOOL_RESPONSE } from "../../__fixtures__/tools_response";
import { ToolGroup } from "../../services/refact";

const TOOLS_PER_PAGE = 5;

export const AgentCapabilities = () => {
  const [selectedToolGroup, setSelectedToolGroup] = useState<ToolGroup | null>(
    null,
  );
  const [toolGroupPage, setToolGroupPage] = useState(0);

  useEffect(() => {
    setToolGroupPage(0);
  }, [selectedToolGroup]);

  const isPatchAutomatic = useAppSelector(selectAutomaticPatch);
  const isAgentRollbackEnabled = useAppSelector(selectCheckpointsEnabled);
  const areFollowUpsEnabled = useAppSelector(selectAreFollowUpsEnabled);
  const agenticFeatures = useMemo(() => {
    return [
      {
        name: "Auto-patch",
        enabled: isPatchAutomatic,
        switcher: <ApplyPatchSwitch />,
      },
      {
        name: "Files rollback",
        enabled: isAgentRollbackEnabled,
        switcher: <AgentRollbackSwitch />,
      },
      {
        name: "Follow-Ups",
        enabled: areFollowUpsEnabled,
        switcher: <FollowUpsSwitch />,
      },
    ];
  }, [isPatchAutomatic, isAgentRollbackEnabled, areFollowUpsEnabled]);

  return (
    <Flex mb="2" gap="2" align="center">
      <Popover.Root defaultOpen>
        <Popover.Trigger>
          <IconButton variant="soft" size="1">
            <MixerVerticalIcon />
          </IconButton>
        </Popover.Trigger>
        <Popover.Content side="top" alignOffset={-10} sideOffset={20}>
          <Flex gap="2" direction="column">
            {agenticFeatures.map((feature) => {
              return feature.switcher;
            })}
            <Separator size="4" mt="2" mb="1" />
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
                    <Flex direction="column" gap="2">
                      {STUB_TOOL_RESPONSE.map((toolGroup) => (
                        <Card
                          key={toolGroup.name}
                          size="1"
                          onClick={() => setSelectedToolGroup(toolGroup)}
                          style={{
                            cursor: "pointer",
                          }}
                        >
                          <Heading as="h4" size="1">
                            <Flex align="center" justify="between">
                              <Flex as="span" align="center" gap="1">
                                {/* TODO: do we want to differentiate somehow groups by their source types or only tools themselves? */}
                                {toolGroup.name}
                                <HoverCard.Root>
                                  <HoverCard.Trigger>
                                    <QuestionMarkCircledIcon
                                      style={{ marginLeft: 4 }}
                                    />
                                  </HoverCard.Trigger>
                                  <HoverCard.Content size="1">
                                    <Text as="p" size="2">
                                      {toolGroup.description}
                                    </Text>
                                  </HoverCard.Content>
                                </HoverCard.Root>
                              </Flex>
                              <Flex align="center" gap="1">
                                <Tooltip content="Indicates how many tools the group contains">
                                  <Badge color="indigo">
                                    {toolGroup.tools.length}
                                  </Badge>
                                </Tooltip>
                                <ChevronRightIcon />
                              </Flex>
                            </Flex>
                          </Heading>
                        </Card>
                      ))}
                    </Flex>
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
                      <Flex direction="column" gap="3">
                        {selectedToolGroup.tools
                          .slice(
                            toolGroupPage * TOOLS_PER_PAGE,
                            (toolGroupPage + 1) * TOOLS_PER_PAGE,
                          )
                          .map((tool) => (
                            <Flex
                              key={tool.spec.name}
                              align="center"
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
                              <Switch size="1" />
                            </Flex>
                          ))}
                      </Flex>
                      {/* Pagination controls */}
                      {selectedToolGroup.tools.length > TOOLS_PER_PAGE && (
                        <Flex gap="2" mt="2" align="center" justify="between">
                          <Button
                            size="1"
                            disabled={toolGroupPage === 0}
                            onClick={() => setToolGroupPage((p) => p - 1)}
                          >
                            Prev
                          </Button>
                          <Text size="1">
                            Page {toolGroupPage + 1} of{" "}
                            {Math.ceil(
                              selectedToolGroup.tools.length / TOOLS_PER_PAGE,
                            )}
                          </Text>
                          <Button
                            size="1"
                            disabled={
                              toolGroupPage >=
                              Math.ceil(
                                selectedToolGroup.tools.length / TOOLS_PER_PAGE,
                              ) -
                                1
                            }
                            onClick={() => setToolGroupPage((p) => p + 1)}
                          >
                            Next
                          </Button>
                        </Flex>
                      )}
                    </Flex>
                  </motion.div>
                )}
              </AnimatePresence>
            </Flex>
          </Flex>
        </Popover.Content>
      </Popover.Root>
      <Text size="2">
        Enabled Features:
        <Text color="gray">
          {" "}
          {agenticFeatures
            .filter((feature) => feature.enabled)
            .map((feature) => feature.name)
            .join(", ") || "None"}
        </Text>
      </Text>
      <HoverCard.Root>
        <HoverCard.Trigger>
          <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
        </HoverCard.Trigger>
        <HoverCard.Content size="2" maxWidth="280px">
          <Text as="p" size="2">
            Here you can control special features affecting Agent behaviour
          </Text>
        </HoverCard.Content>
      </HoverCard.Root>
    </Flex>
  );
};
