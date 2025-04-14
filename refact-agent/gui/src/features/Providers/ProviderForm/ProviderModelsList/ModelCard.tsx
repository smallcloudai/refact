import {
  Badge,
  Button,
  Card,
  Dialog,
  DropdownMenu,
  Flex,
  IconButton,
  Text,
  TextField,
} from "@radix-ui/themes";
import { DotsVerticalIcon } from "@radix-ui/react-icons";
import {
  CodeChatModel,
  EmbeddingModel,
  isCodeChatModel,
  isEmbeddingModel,
  ModelType,
  SimplifiedModel,
} from "../../../../services/refact";
import { FC, useState, useEffect } from "react";
import { useGetModelConfiguration } from "../../../../hooks/useModelsQuery";

export type ModelCardProps = {
  model: SimplifiedModel;
  providerName: string;
  modelType: ModelType;
};

export const ModelCard: FC<ModelCardProps> = ({
  model,
  modelType,
  providerName,
}) => {
  const { enabled, name, removable } = model;
  const [isEditingModel, setIsEditingModel] = useState(false);

  const handleDisableModel = () => {
    // eslint-disable-next-line no-console
    console.log("disable model");
  };

  const handleModelSettings = () => {
    // eslint-disable-next-line no-console
    console.log("model settings");
    setIsEditingModel(true);
  };

  return (
    <Card>
      {isEditingModel && (
        <ModelCardPopup
          isOpen={isEditingModel}
          setIsOpen={setIsEditingModel}
          modelName={name}
          modelType={modelType}
          providerName={providerName}
        />
      )}
      <Flex align="center" justify="between">
        <Flex gap="2" align="center">
          <Text as="span" size="2">
            {name}
          </Text>
          <Badge size="1" color={enabled ? "green" : "gray"}>
            {enabled ? "Active model" : "Disabled model"}
          </Badge>
        </Flex>
        <DropdownMenu.Root>
          <DropdownMenu.Trigger>
            <IconButton size="1" variant="outline" color="gray">
              <DotsVerticalIcon />
            </IconButton>
          </DropdownMenu.Trigger>
          <DropdownMenu.Content side="bottom" align="end" size="1">
            <DropdownMenu.Item onClick={handleModelSettings}>
              Edit model&apos;s settings
            </DropdownMenu.Item>
            <DropdownMenu.Item onClick={handleDisableModel}>
              {enabled ? "Disable model" : "Enable model"}
            </DropdownMenu.Item>
            <DropdownMenu.Item
              onClick={handleDisableModel}
              color="red"
              disabled={!removable}
              title={
                removable
                  ? "Remove model from the list of models"
                  : `${name} is not removable model`
              }
            >
              Remove model
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Root>
      </Flex>
    </Card>
  );
};

type ModelCardPopupProps = {
  isOpen: boolean;
  setIsOpen: (state: boolean) => void;
  modelName: string;
  modelType: ModelType;
  providerName: string;
};

const ModelCardPopup: FC<ModelCardPopupProps> = ({
  isOpen,
  setIsOpen,
  modelName,
  modelType,
  providerName,
}) => {
  const { data: modelData, isSuccess } = useGetModelConfiguration({
    modelName,
    modelType,
    providerName,
  });

  // State for editing model configuration
  const [editedModelData, setEditedModelData] = useState(modelData);

  // Update local state when model data loads
  useEffect(() => {
    setEditedModelData(modelData);
  }, [modelData]);

  if (!isSuccess && !modelData) {
    return null;
  }

  // Function to toggle boolean capabilities
  const toggleCapability = (key: string) => {
    if (!editedModelData) return;

    setEditedModelData({
      ...editedModelData,
      [key]: !editedModelData[key as keyof typeof editedModelData],
    });
  };

  // Common fields for all model types
  const renderCommonFields = () => (
    <>
      <label>
        <Text as="div" size="2" mb="1" weight="bold">
          Name
        </Text>
        <TextField.Root
          defaultValue={editedModelData?.name}
          placeholder="Model name"
        />
      </label>
      <label>
        <Text as="div" size="2" mb="1" weight="bold">
          Context Window (n_ctx)
        </Text>
        <TextField.Root
          defaultValue={editedModelData?.n_ctx.toString()}
          placeholder="Context window size"
          type="number"
        />
      </label>
      <label>
        <Text as="div" size="2" mb="1" weight="bold">
          Tokenizer
        </Text>
        <TextField.Root
          defaultValue={editedModelData?.tokenizer}
          placeholder="Tokenizer name"
        />
      </label>
    </>
  );

  // Chat model specific fields
  const renderChatModelFields = () => {
    if (!editedModelData) return;
    return (
      <>
        <label>
          <Text as="div" size="2" mb="1" weight="bold">
            Default Temperature
          </Text>
          <TextField.Root
            defaultValue={
              (
                editedModelData as CodeChatModel
              ).default_temperature?.toString() ?? ""
            }
            placeholder="Default temperature"
            type="number"
          />
        </label>
        <Flex direction="column" gap="2">
          <Text as="div" size="2" weight="bold">
            Capabilities
          </Text>
          <Flex gap="2" wrap="wrap">
            <Badge
              color={
                (editedModelData as CodeChatModel).supports_tools
                  ? "green"
                  : "gray"
              }
              onClick={() => toggleCapability("supports_tools")}
              style={{ cursor: "pointer" }}
            >
              Tools{" "}
              {(editedModelData as CodeChatModel).supports_tools ? "✓" : "✗"}
            </Badge>
            <Badge
              color={
                (editedModelData as CodeChatModel).supports_multimodality
                  ? "green"
                  : "gray"
              }
              onClick={() => toggleCapability("supports_multimodality")}
              style={{ cursor: "pointer" }}
            >
              Multimodality{" "}
              {(editedModelData as CodeChatModel).supports_multimodality
                ? "✓"
                : "✗"}
            </Badge>
            <Badge
              color={
                (editedModelData as CodeChatModel).supports_clicks
                  ? "green"
                  : "gray"
              }
              onClick={() => toggleCapability("supports_clicks")}
              style={{ cursor: "pointer" }}
            >
              Clicks{" "}
              {(editedModelData as CodeChatModel).supports_clicks ? "✓" : "✗"}
            </Badge>
            <Badge
              color={
                (editedModelData as CodeChatModel).supports_agent
                  ? "green"
                  : "gray"
              }
              onClick={() => toggleCapability("supports_agent")}
              style={{ cursor: "pointer" }}
            >
              Agent{" "}
              {(editedModelData as CodeChatModel).supports_agent ? "✓" : "✗"}
            </Badge>
            <Badge
              color={
                (editedModelData as CodeChatModel).supports_reasoning
                  ? "green"
                  : "gray"
              }
              // Support reasoning is a string enum, not a boolean
              // So we don't toggle it for now
            >
              Reasoning{" "}
              {(editedModelData as CodeChatModel).supports_reasoning ?? "✗"}
            </Badge>
          </Flex>
        </Flex>
      </>
    );
  };

  // Embedding model specific fields
  const renderEmbeddingModelFields = () => {
    if (!editedModelData) return;

    return (
      <>
        <label>
          <Text as="div" size="2" mb="1" weight="bold">
            Embedding Size
          </Text>
          <TextField.Root
            defaultValue={(
              editedModelData as EmbeddingModel
            ).embedding_size.toString()}
            placeholder="Embedding size"
            type="number"
          />
        </label>
        <label>
          <Text as="div" size="2" mb="1" weight="bold">
            Rejection Threshold
          </Text>
          <TextField.Root
            defaultValue={(
              editedModelData as EmbeddingModel
            ).rejection_threshold.toString()}
            placeholder="Rejection threshold"
            type="number"
          />
        </label>
        <label>
          <Text as="div" size="2" mb="1" weight="bold">
            Embedding Batch
          </Text>
          <TextField.Root
            defaultValue={(
              editedModelData as EmbeddingModel
            ).embedding_batch.toString()}
            placeholder="Embedding batch"
            type="number"
          />
        </label>
      </>
    );
  };

  return (
    <Dialog.Root open={isOpen} onOpenChange={setIsOpen}>
      <Dialog.Content maxWidth="450px">
        <Dialog.Title>Model Configuration</Dialog.Title>
        <Dialog.Description size="2" mb="4">
          Make changes to {modelName} ({modelType} model)
        </Dialog.Description>

        <Flex direction="column" gap="3">
          {renderCommonFields()}
          {isCodeChatModel(modelData) && renderChatModelFields()}
          {isEmbeddingModel(modelData) && renderEmbeddingModelFields()}
          {/* Completion model has no unique fields beyond common ones */}
        </Flex>

        <Flex gap="3" mt="4" justify="end">
          <Dialog.Close>
            <Button variant="soft" color="gray">
              Cancel
            </Button>
          </Dialog.Close>
          <Button
            onClick={() => {
              // eslint-disable-next-line no-console
              console.log(`update ${modelName} model, data: `, editedModelData);
              setIsOpen(false);
            }}
          >
            Save
          </Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};
