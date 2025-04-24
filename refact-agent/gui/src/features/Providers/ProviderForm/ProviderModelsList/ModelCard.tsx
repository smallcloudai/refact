import { useCallback, type FC } from "react";
import classNames from "classnames";
import {
  Badge,
  Card,
  DropdownMenu,
  Flex,
  IconButton,
  Text,
} from "@radix-ui/themes";
import { DotsVerticalIcon } from "@radix-ui/react-icons";

import { ModelCardPopup } from "./components/ModelCardPopup";
import { useModelDialogState } from "./hooks/useModelDialogState";

import type { ModelType, SimplifiedModel } from "../../../../services/refact";

import styles from "./ModelCard.module.css";
import { useEventsBusForIDE } from "../../../../hooks";

export type ModelCardProps = {
  model: SimplifiedModel;
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
        <Flex gap="2" align="center">
          <Text as="span" size="2">
            {name}
          </Text>
          <Badge size="1" color={enabled ? "green" : "gray"}>
            {enabled ? "Active" : "Inactive"}
          </Badge>
        </Flex>

        <DropdownMenu.Root open={dropdownOpen} onOpenChange={setDropdownOpen}>
          <DropdownMenu.Trigger>
            <IconButton size="1" variant="outline" color="gray">
              <DotsVerticalIcon />
            </IconButton>
          </DropdownMenu.Trigger>
          <DropdownMenu.Content side="bottom" align="end" size="1">
            <DropdownMenu.Item
              onClick={openDialogSafely}
              disabled={isReadonlyProvider || isSavingModel}
            >
              Edit model&apos;s settings
            </DropdownMenu.Item>
            <DropdownMenu.Item
              onClick={() => void handleToggleModelEnabledState(model)}
              disabled={isReadonlyProvider || isSavingModel}
            >
              {enabled ? "Disable model" : "Enable model"}
            </DropdownMenu.Item>
            {modelType === "completion" && (
              <DropdownMenu.Item onClick={handleSetCompletionModelForIDE}>
                Use as completion model in IDE
              </DropdownMenu.Item>
            )}
            {removable ? (
              <DropdownMenu.Item
                onClick={() => void handleRemoveModel({ model })}
                color="red"
                disabled={isSavingModel}
                title={"Remove model from the list of models"}
              >
                Remove model
              </DropdownMenu.Item>
            ) : (
              <DropdownMenu.Item
                onClick={() => void handleResetModel(model)}
                color="red"
                disabled={isSavingModel || !user_configured}
                title={"Reset model from the list of models"}
              >
                Reset model
              </DropdownMenu.Item>
            )}
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </Flex>
    </Card>
  );
};
