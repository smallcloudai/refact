import {
  MixerVerticalIcon,
  QuestionMarkCircledIcon,
} from "@radix-ui/react-icons";
import {
  Flex,
  HoverCard,
  IconButton,
  Popover,
  Separator,
  Text,
} from "@radix-ui/themes";
import {
  AgentRollbackSwitch,
  ApplyPatchSwitch,
  FollowUpsSwitch,
} from "../ChatControls";
import { useAppSelector } from "../../../hooks";
import {
  selectAreFollowUpsEnabled,
  selectAutomaticPatch,
  selectCheckpointsEnabled,
} from "../../../features/Chat";
import { useMemo } from "react";
import { ToolGroups } from "./ToolGroups";

export const AgentCapabilities = () => {
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
      <Popover.Root>
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
            <ToolGroups />
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
