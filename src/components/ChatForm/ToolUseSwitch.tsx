import { Flex, SegmentedControl, Text, HoverCard } from "@radix-ui/themes";
import { ToolUse } from "../../features/Chat";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";

type ToolUseSwitchProps = {
  toolUse: ToolUse;
  setToolUse: (toolUse: ToolUse) => void;
};

export const ToolUseSwitch = ({ toolUse, setToolUse }: ToolUseSwitchProps) => {
  return (
    <Flex direction="column" gap="3" mb="2" align="start">
      <Text size="1">How fast do you want the answer:</Text>
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
              Provides a fast response with less detail.
            </Text>
            <Text as="div" mt="2" weight="bold">
              Explore
            </Text>
            <Text as="p" size="2">
              Explores the topic in more depth.
            </Text>
            <Text as="div" mt="2" weight="bold">
              Agent
            </Text>
            <Text as="p" size="2">
              Acts as an agent to perform tasks.
            </Text>
          </HoverCard.Content>
        </HoverCard.Root>
      </Flex>
    </Flex>
  );
};
