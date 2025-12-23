import React from "react";
import { Flex, SegmentedControl, Text, HoverCard } from "@radix-ui/themes";
import { ToolUse } from "../../features/Chat/Thread";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";

type ToolUseSwitchProps = {
  toolUse: ToolUse;
  setToolUse: (toolUse: ToolUse) => void;
};

export const ToolUseSwitch = React.forwardRef<
  HTMLDivElement,
  ToolUseSwitchProps
>(({ toolUse, setToolUse }, ref) => {
  return (
    <Flex direction="column" gap="3" mb="2" align="start" ref={ref}>
      <Text size="2">How fast do you want the answer:</Text>
      <Flex direction="row" gap="1" align="center">
        <SegmentedControl.Root
          defaultValue="quick"
          value={toolUse}
          onValueChange={(x) => {
            setToolUse(x as ToolUse);
          }}
        >
          <SegmentedControl.Item value="quick">Quick</SegmentedControl.Item>
          <SegmentedControl.Item value="explore">Explore</SegmentedControl.Item>
          <SegmentedControl.Item value="agent">Agent</SegmentedControl.Item>
        </SegmentedControl.Root>
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content size="2" maxWidth="280px">
            <Text weight="bold">Quick</Text>
            <Text as="p" size="2">
              The model doesn&apos;t have access to any tools and answers
              immediately. You still can provide context using @-commands, try
              @help.
            </Text>
            <Text as="div" mt="2" weight="bold">
              Explore
            </Text>
            <Text as="p" size="2">
              The model has access to exploration tools and collects the
              necessary context for you.
            </Text>
            <Text as="div" mt="2" weight="bold">
              Agent
            </Text>
            <Text as="p" size="2">
              The model has agent capabilities, might take a long time to
              respond. For example it can provide a high-quality context to
              solve a problem.
            </Text>
          </HoverCard.Content>
        </HoverCard.Root>
      </Flex>
    </Flex>
  );
});

ToolUseSwitch.displayName = "ToolUseSwitch";
