import React from "react";
import { Flex, Text, Switch, HoverCard } from "@radix-ui/themes";
import { QuestionMarkCircledIcon } from "@radix-ui/react-icons";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { selectBoostReasoning, setBoostReasoning } from "../../features/Chat/Thread";

export const BoostReasoningSwitch: React.FC = () => {
  const dispatch = useAppDispatch();
  const isBoostReasoningEnabled = useAppSelector(selectBoostReasoning);

  const handleReasoningChange = (checked: boolean) => {
    dispatch(setBoostReasoning(checked));
  };

  return (
    <Flex
      gap="4"
      align="center"
      wrap="wrap"
      flexGrow="1"
      flexShrink="0"
      width="100%"
      justify="between"
    >
      <Text size="2" mr="auto">
        Boost reasoning
      </Text>
      <Flex gap="2" align="center">
        <Switch
          size="1"
          title="Enable/disable boosted reasoning for this model"
          checked={isBoostReasoningEnabled}
          onCheckedChange={handleReasoningChange}
        />
        <HoverCard.Root>
          <HoverCard.Trigger>
            <QuestionMarkCircledIcon style={{ marginLeft: 4 }} />
          </HoverCard.Trigger>
          <HoverCard.Content size="2" maxWidth="280px">
            <Text as="p" size="2">
              When enabled, the model will use enhanced reasoning capabilities which may improve problem-solving for complex tasks.
            </Text>
          </HoverCard.Content>
        </HoverCard.Root>
      </Flex>
    </Flex>
  );
};