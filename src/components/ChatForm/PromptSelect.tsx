import React from "react";
import { Flex, Text } from "@radix-ui/themes";
import { Root, Trigger, Content, Item } from "../Select";
import type { SystemPrompts } from "../../services/refact";

export type PromptSelectProps = {
  value: string;
  onChange: (value: string) => void;
  prompts: SystemPrompts;
  disabled?: boolean;
};

export const PromptSelect: React.FC<PromptSelectProps> = ({
  value,
  prompts,
  onChange,
  disabled,
}) => {
  const promptKeysAndValues = Object.entries(prompts);
  if (promptKeysAndValues.length === 0) return null;
  return (
    <Flex gap="2" align="center" wrap="wrap">
      <Text size="2">System Prompt:</Text>
      <Root
        name="system prompt"
        disabled={disabled}
        onValueChange={onChange}
        value={value}
        size="1"
      >
        <Trigger title={value} />
        <Content>
          {Object.entries(prompts).map(([key, value]) => {
            return (
              <Item key={key} value={value.text} title={value.text}>
                {key}
              </Item>
            );
          })}
        </Content>
      </Root>
    </Flex>
  );
};
