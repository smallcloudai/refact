import { Flex, SegmentedControl, Text } from "@radix-ui/themes";
import { ToolUse } from "../../features/Chat";

type ToolUseSwitchProps = {
  toolUse: ToolUse;
  setToolUse: (toolUse: ToolUse) => void;
};

export const ToolUseSwitch = ({ toolUse, setToolUse }: ToolUseSwitchProps) => {
  return (
    <Flex direction="column" gap="8px" align="start">
      <Text size="2">How fast you want the answer:</Text>
      <SegmentedControl.Root
        defaultValue="quick"
        value={toolUse}
        onValueChange={(x) => {
          console.log({ x });
          setToolUse(x as ToolUse);
        }}
      >
        <SegmentedControl.Item value="quick">Quick</SegmentedControl.Item>
        <SegmentedControl.Item value="explore">Explore</SegmentedControl.Item>
        <SegmentedControl.Item value="agent">Agent</SegmentedControl.Item>
      </SegmentedControl.Root>
    </Flex>
  );
};
