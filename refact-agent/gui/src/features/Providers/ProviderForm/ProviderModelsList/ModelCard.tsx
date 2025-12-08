import { useCallback, useMemo, type FC } from "react";
import classNames from "classnames";
import {
  Badge,
  Card,
  DropdownMenu,
  Flex,
  IconButton,
  Text,
  Tooltip,
} from "@radix-ui/themes";
import { DotsVerticalIcon } from "@radix-ui/react-icons";

import { ModelCardPopup } from "./components/ModelCardPopup";
import { CapabilityIcons } from "./components/CapabilityIcons";
import { useModelDialogState } from "./hooks/useModelDialogState";

import type { ModelType } from "../../../../services/refact";
import type { UiModel } from "./utils/groupModelsWithPricing";

import styles from "./ModelCard.module.css";
import { useEventsBusForIDE } from "../../../../hooks";

export type ModelCardProps = {
  model: UiModel;
  providerName: string;
  modelType: ModelType;
  isReadonlyProvider: boolean;
  currentModelNames: string[];
};

/**
 * Card component that displays model information and provides access to model settings
 */
export const ModelCard: FC<ModelCardProps> = ({
  model,
  modelType,
  providerName,
  isReadonlyProvider,
  currentModelNames,
}) => {
  const { enabled, name, removable, user_configured } = model;
  const {
    isOpen: dialogOpen,
    setIsOpen: setDialogOpen,
    dropdownOpen,
    setDropdownOpen,
    openDialogSafely,
    isSavingModel,
    handleToggleModelEnabledState,
    handleRemoveModel,
    handleResetModel,
    handleSaveModel,
    handleUpdateModel,
  } = useModelDialogState({
    initialState: false,
    modelType,
    providerName,
  });

  const { setCodeCompletionModel } = useEventsBusForIDE();

  const handleSetCompletionModelForIDE = useCallback(() => {
    const formattedModelName = `${providerName}/${model.name}`;
    setCodeCompletionModel(formattedModelName);
  }, [model, providerName, setCodeCompletionModel]);

  const dropdownOptions = useMemo(() => {
    const shouldOptionsBeDisabled = isReadonlyProvider || isSavingModel;
    return [
      {
        label: "Edit model's settings",
        onClick: openDialogSafely,
        visible: !shouldOptionsBeDisabled,
      },
      {
        label: enabled ? "Disable model" : "Enable model",
        onClick: () => void handleToggleModelEnabledState(model),
        visible: !shouldOptionsBeDisabled,
      },
      {
        label: "Reset model",
        onClick: () => void handleResetModel(model),
        visible: !removable && user_configured,
      },
      {
        label: "Remove model",
        onClick: () => void handleRemoveModel({ model }),
        visible: removable,
      },
      {
        label: "Use as completion model in IDE",
        onClick: handleSetCompletionModelForIDE,
        visible: modelType === "completion",
      },
    ];
  }, [
    isReadonlyProvider,
    isSavingModel,
    enabled,
    removable,
    user_configured,
    model,
    modelType,
    openDialogSafely,
    handleToggleModelEnabledState,
    handleResetModel,
    handleRemoveModel,
    handleSetCompletionModelForIDE,
  ]);

  const dropdownOptionsCount = useMemo(() => {
    return dropdownOptions.filter((option) => option.visible).length;
  }, [dropdownOptions]);

  return (
    <Card className={classNames({ [styles.disabledCard]: isSavingModel })}>
      {dialogOpen && (
        <ModelCardPopup
          minifiedModel={model}
          isOpen={dialogOpen}
          isSaving={isSavingModel}
          setIsOpen={setDialogOpen}
          modelName={name}
          modelType={modelType}
          providerName={providerName}
          onSave={handleSaveModel}
          onUpdate={handleUpdateModel}
          isRemovable={removable}
          currentModelNames={currentModelNames}
        />
      )}

      <Flex align="center" justify="between">
        <Flex direction="column" gap="1" style={{ flex: 1, minWidth: 0 }}>
          <Flex gap="2" align="center" wrap="wrap">
            <Text as="span" size="2" weight="medium">
              {name}
            </Text>
            <Badge size="1" color={enabled ? "green" : "gray"}>
              {enabled ? "Active" : "Inactive"}
            </Badge>
          </Flex>

          <Flex gap="2" align="center" wrap="wrap">
            {model.pricingLabel && (
              <Tooltip content="Price per 1M tokens (prompt/output)">
                <Text as="span" size="1" color="gray">
                  💰 {model.pricingLabel}
                </Text>
              </Tooltip>
            )}
            {model.nCtxLabel && (
              <Tooltip
                content={`Context window: ${model.nCtx?.toLocaleString()} tokens`}
              >
                <Text as="span" size="1" color="gray">
                  📏 {model.nCtxLabel}
                </Text>
              </Tooltip>
            )}
            {model.capabilities && (
              <CapabilityIcons capabilities={model.capabilities} size="1" />
            )}
          </Flex>
        </Flex>

        {dropdownOptionsCount > 0 && (
          <DropdownMenu.Root open={dropdownOpen} onOpenChange={setDropdownOpen}>
            <DropdownMenu.Trigger>
              <IconButton size="1" variant="outline" color="gray">
                <DotsVerticalIcon />
              </IconButton>
            </DropdownMenu.Trigger>
            <DropdownMenu.Content side="bottom" align="end" size="1">
              {dropdownOptions.map(({ label, visible, onClick }) => {
                if (!visible) return null;
                return (
                  <DropdownMenu.Item
                    key={label}
                    onClick={onClick}
                    title={label}
                  >
                    {label}
                  </DropdownMenu.Item>
                );
              })}
            </DropdownMenu.Content>
          </DropdownMenu.Root>
        )}
      </Flex>
    </Card>
  );
};
