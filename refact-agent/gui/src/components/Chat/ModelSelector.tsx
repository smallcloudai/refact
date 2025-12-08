import React, { useMemo } from "react";
import { Select, Text, Flex } from "@radix-ui/themes";
import { useCapsForToolUse } from "../../hooks";
import { RichModelSelectItem } from "../Select/RichModelSelectItem";
import { enrichAndGroupModels } from "../../utils/enrichModels";
import styles from "../Select/select.module.css";

export type ModelSelectorProps = {
  disabled?: boolean;
};

export const ModelSelector: React.FC<ModelSelectorProps> = ({ disabled }) => {
  const capsForToolUse = useCapsForToolUse();

  const groupedModels = useMemo(
    () =>
      enrichAndGroupModels(
        capsForToolUse.usableModelsForPlan,
        capsForToolUse.data,
      ),
    [capsForToolUse.usableModelsForPlan, capsForToolUse.data],
  );

  const currentModelName = capsForToolUse.currentModel.replace(/^refact\//, "");

  if (!capsForToolUse.data || groupedModels.length === 0) {
    return (
      <Text size="1" color="gray">
        model: {currentModelName}
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
          title={
            disabled
              ? "Cannot change model while streaming"
              : "Click to change model"
          }
          style={{
            cursor: disabled ? "not-allowed" : "pointer",
            padding: "0 4px",
            minHeight: "20px",
            height: "20px",
            opacity: disabled ? 0.5 : 1,
          }}
        />
        <Select.Content position="popper">
          {groupedModels.map((group) => (
            <Select.Group key={group.provider}>
              <Select.Label>{group.displayName}</Select.Label>
              {group.models.map((model) => (
                <Select.Item
                  key={model.value}
                  value={model.value}
                  disabled={model.disabled}
                  textValue={model.displayName}
                >
                  <span className={styles.trigger_only}>{model.displayName}</span>
                  <span className={styles.dropdown_only}>
                    <RichModelSelectItem
                      displayName={model.displayName}
                      pricing={model.pricing}
                      nCtx={model.nCtx}
                      capabilities={model.capabilities}
                      isDefault={model.isDefault}
                      isThinking={model.isThinking}
                      isLight={model.isLight}
                    />
                  </span>
                </Select.Item>
              ))}
            </Select.Group>
          ))}
        </Select.Content>
      </Select.Root>
    </Flex>
  );
};
