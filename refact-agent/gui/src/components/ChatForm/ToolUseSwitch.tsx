import React, { useMemo } from "react";
import { Flex, Text } from "@radix-ui/themes";
import { ToolUse } from "../../features/Chat/Thread";
import { Select, type ItemProps } from "../Select";

type ToolUseSwitchProps = {
  toolUse: ToolUse;
  setToolUse: (toolUse: ToolUse) => void;
};

export const ToolUseSwitch = React.forwardRef<
  HTMLDivElement,
  ToolUseSwitchProps
>(({ toolUse, setToolUse }, ref) => {
  const options: ItemProps[] = useMemo(() => [
    {
      value: "quick",
      children: "Quick",
      tooltip: (
        <div>
          <Text weight="bold" as="div">Quick</Text>
          <Text as="p" size="2">
            The model doesn&apos;t have access to any tools and answers
            immediately. You still can provide context using @-commands, try
            @help.
          </Text>
        </div>
      ),
    },
    {
      value: "explore",
      children: "Explore",
      tooltip: (
        <div>
          <Text weight="bold" as="div">Explore</Text>
          <Text as="p" size="2">
            The model has access to exploration tools and collects the
            necessary context for you.
          </Text>
        </div>
      ),
    },
    {
      value: "agent",
      children: "Agent",
      tooltip: (
        <div>
          <Text weight="bold" as="div">Agent</Text>
          <Text as="p" size="2">
            The model has agent capabilities, might take a long time to
            respond. For example it can provide a high-quality context to
            solve a problem.
          </Text>
        </div>
      ),
    },
  ], []);

  const handleChange = (value: string) => {
    setToolUse(value as ToolUse);
  };

  return (
    <Flex direction="column" gap="3" mb="2" align="start" ref={ref}>
      <Text size="2">How fast do you want the answer:</Text>
      <Select
        title="Response speed mode"
        options={options}
        value={toolUse}
        onChange={handleChange}
        contentPosition="popper"
      />
    </Flex>
  );
});

ToolUseSwitch.displayName = "ToolUseSwitch";
