import React, { useMemo } from "react";
import { Select, Text, Flex } from "@radix-ui/themes";
import { useCapsForToolUse } from "../../hooks";

export type ModelSelectorProps = {
  disabled?: boolean;
};

export const ModelSelector: React.FC<ModelSelectorProps> = ({ disabled }) => {
  const capsForToolUse = useCapsForToolUse();

  const modelOptions = useMemo(() => {
    return capsForToolUse.usableModelsForPlan.map((model) => ({
      value: model.value,
      label: model.textValue,
      disabled: model.disabled,
    }));
  }, [capsForToolUse.usableModelsForPlan]);

  if (!capsForToolUse.data || modelOptions.length === 0) {
    return (
      <Text size="1" color="gray">
        model: {capsForToolUse.currentModel}
      </Text>
    );
  }

  return (
    <Flex align="center" gap="1" style={{ height: "20px" }}>
      <Text size="1" color="gray" style={{ lineHeight: "20px" }}>
        model:
      </Text>
      <Select.Root
        value={capsForToolUse.currentModel}
        onValueChange={capsForToolUse.setCapModel}
        disabled={disabled}
        size="1"
      >
        <Select.Trigger
          variant="ghost"
          title={disabled ? "Cannot change model while streaming" : "Click to change model"}
          style={{
            cursor: disabled ? "not-allowed" : "pointer",
            padding: "0 4px",
            minHeight: "20px",
            height: "20px",
            opacity: disabled ? 0.5 : 1,
          }}
        />
        <Select.Content position="popper">
          {modelOptions.map((option) => (
            <Select.Item
              key={option.value}
              value={option.value}
              disabled={option.disabled}
            >
              {option.label}
            </Select.Item>
          ))}
        </Select.Content>
      </Select.Root>
    </Flex>
  );
};
