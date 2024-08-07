import React, { useCallback, useMemo } from "react";
import { Flex, Text } from "@radix-ui/themes";
import { Root, Trigger, Content, Item } from "../Select";
import type { SystemPrompts } from "../../services/refact";

export type PromptSelectProps = {
  value: SystemPrompts;
  onChange: (value: SystemPrompts) => void;
  prompts: SystemPrompts;
  disabled?: boolean;
};

export const PromptSelect: React.FC<PromptSelectProps> = ({
  value,
  prompts,
  onChange,
  disabled,
}) => {
  // TODO: just use the hooks here
  const promptKeysAndValues = Object.entries(prompts);
  const handleChange = useCallback(
    (key: string) => {
      const item = promptKeysAndValues.find((p) => p[0] === key);
      if (!item) return;
      const prompt = { [item[0]]: item[1] };
      onChange(prompt);
    },
    [onChange, promptKeysAndValues],
  );
  const val = useMemo(() => Object.keys(value)[0] ?? "default", [value]);
  if (promptKeysAndValues.length === 0) return null;

  return (
    <Flex gap="2" align="center" wrap="wrap">
      <Text size="2">System Prompt:</Text>
      <Root
        name="system prompt"
        disabled={disabled}
        onValueChange={handleChange}
        value={val}
        size="1"
      >
        <Trigger title={val} />
        <Content>
          {Object.entries(prompts).map(([key, value]) => {
            return (
              <Item
                key={key}
                value={key}
                title={value.description || value.text}
              >
                {key}
              </Item>
            );
          })}
        </Content>
      </Root>
    </Flex>
  );
};
