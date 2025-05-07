import React from "react";
import { Flex, Text, HoverCard } from "@radix-ui/themes";
import { ToolUse } from "../../features/Chat/Thread";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import { Select } from "../Select";

type ToolUseSwitchProps = {
  toolUse: ToolUse;
  setToolUse: (toolUse: ToolUse) => void;
};

export const ToolUseSwitch = React.forwardRef<
  HTMLDivElement,
  ToolUseSwitchProps
>(({ toolUse, setToolUse }, ref) => {
  const options = [
    { value: "quick", textValue: "Quick" },
    { value: "explore", textValue: "Explore" },
    { value: "agent", textValue: "Agent" }
  ];

  return (
    <Flex direction="row" gap="2" mb="2" align="center" ref={ref}>
      <Flex align="center" gap="1">
        <Text size="2">⚡ Response Mode:</Text>
      </Flex>
      <Flex direction="row" gap="1" align="center">
        <Select
          title="Response Mode"
          options={options}
          value={toolUse}
          onChange={(value) => setToolUse(value as ToolUse)}
        />
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
