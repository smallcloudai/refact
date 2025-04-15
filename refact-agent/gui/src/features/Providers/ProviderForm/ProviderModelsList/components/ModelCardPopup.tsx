import type { FC } from "react";
import { useState, useEffect, useCallback, ChangeEvent } from "react";
import isEqual from "lodash.isequal";
import { Button, Dialog, Flex, Text } from "@radix-ui/themes";

import { useGetModelConfiguration } from "../../../../../hooks/useModelsQuery";

import { FormField } from "./FormField";
import { CapabilityBadge } from "./CapabilityBadge";

import type {
  CodeChatModel,
  EmbeddingModel,
  Model,
  ModelType,
} from "../../../../../services/refact";
import {
  isCodeChatModel,
  isEmbeddingModel,
} from "../../../../../services/refact";

export type ModelCardPopupProps = {
  isOpen: boolean;
  isSaving: boolean;
  setIsOpen: (state: boolean) => void;
  onSave: (model: Model) => Promise<boolean>;
  modelName: string;
  modelType: ModelType;
  providerName: string;
};

export const ModelCardPopup: FC<ModelCardPopupProps> = ({
  isOpen,
  isSaving,
  setIsOpen,
  onSave,
  modelName,
  modelType,
  providerName,
}) => {
  const {
    data: modelData,
    isSuccess,
    currentData,
  } = useGetModelConfiguration({
    modelName,
    modelType,
    providerName,
  });
  const [editedModelData, setEditedModelData] = useState(modelData);

  useEffect(() => {
    setEditedModelData(modelData);
  }, [modelData]);

  useEffect(() => {
    if (modelData) {
      setEditedModelData((prev) => {
        if (isEqual(prev, currentData)) return prev;
        return modelData;
      });
    }
  }, [isOpen, modelData, currentData]);

  const handleSave = useCallback(async () => {
    if (!isOpen || !editedModelData) return;

    // eslint-disable-next-line no-console
    console.log(`update ${modelName} model, data: `, editedModelData);
    const isSuccess = await onSave(editedModelData);
    if (!isSuccess) return;
    setTimeout(() => setIsOpen(false), 0);
  }, [isOpen, modelName, editedModelData, setIsOpen, onSave]);

  const handleCancel = useCallback(() => {
    setTimeout(() => setIsOpen(false), 0);
  }, [setIsOpen]);

  const handleDialogChange = useCallback(
    (open: boolean) => {
      setIsOpen(open);
    },
    [setIsOpen],
  );

  // Toggle capability value
  const toggleCapability = (key: string) => {
    if (!editedModelData) return;

    setEditedModelData({
      ...editedModelData,
      [key]: !editedModelData[key as keyof typeof editedModelData],
    });
  };

  const updateFieldByKey = (key: string, value: string | number) => {
    if (!editedModelData) return;
    setEditedModelData({
      ...editedModelData,
      [key]: value,
    });
  };

  if (!isSuccess && !modelData) {
    return null;
  }

  if (!modelData) return null;

  return (
    <Dialog.Root open={isOpen} onOpenChange={handleDialogChange}>
      <Dialog.Content maxWidth="450px">
        <Dialog.Title>Model Configuration</Dialog.Title>
        <Dialog.Description size="2" mb="4">
          Make changes to {modelName} ({modelType} model)
        </Dialog.Description>

        <Flex direction="column" gap="3">
          {editedModelData && (
            <CommonFields
              editedModelData={editedModelData}
              setEditedModelDataByField={updateFieldByKey}
            />
          )}

          {isCodeChatModel(modelData) && (
            <ChatModelFields
              editedModelData={editedModelData as CodeChatModel}
              setEditedModelData={setEditedModelData}
              toggleCapability={toggleCapability}
            />
          )}

          {isEmbeddingModel(modelData) && (
            <EmbeddingModelFields
              editedModelData={editedModelData as EmbeddingModel}
              setEditedModelDataByField={updateFieldByKey}
            />
          )}
        </Flex>

        <Flex gap="3" mt="4" justify="end">
          <Button variant="soft" color="gray" onClick={handleCancel}>
            Cancel
          </Button>
          <Button disabled={isSaving} onClick={() => void handleSave()}>
            {isSaving ? "Saving..." : "Save"}
          </Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};

// Common fields for all model types
type CommonFieldsProps = {
  editedModelData: Model;
  setEditedModelDataByField: (field: string, value: string | number) => void;
};

const CommonFields: FC<CommonFieldsProps> = ({
  editedModelData,
  setEditedModelDataByField,
}) => {
  const getValueByType = (value: string, valueType: string) => {
    if (valueType === "string") return value;
    if (valueType === "number") return parseFloat(value);
    return value;
  };

  const handleFieldValueChange = (
    e: ChangeEvent<HTMLInputElement>,
    field: string,
  ) => {
    const valueType = typeof editedModelData[field as keyof Model];
    const value = getValueByType(e.target.value, valueType);
    setEditedModelDataByField(field, value);
  };

  return (
    <>
      <FormField
        label="Name"
        defaultValue={editedModelData.name}
        placeholder="Model name"
        isDisabled={true}
      />
      <FormField
        label="Context Window (n_ctx)"
        defaultValue={editedModelData.n_ctx.toString()}
        onChange={(e) => handleFieldValueChange(e, "n_ctx")}
        placeholder="Context window size"
        type="number"
      />
      <FormField
        label="Tokenizer"
        defaultValue={editedModelData.tokenizer}
        onChange={(e) => handleFieldValueChange(e, "tokenizer")}
        placeholder="Tokenizer name"
      />
    </>
  );
};

// Chat model specific fields
type ChatModelFieldsProps = {
  editedModelData?: CodeChatModel;
  setEditedModelData: (data: CodeChatModel) => void;
  toggleCapability: (key: string) => void;
};

const ChatModelFields: FC<ChatModelFieldsProps> = ({
  editedModelData,
  setEditedModelData,
  toggleCapability,
}) => {
  const handleTemperatureChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!editedModelData) return;
    const value = parseFloat(e.target.value);
    const digits = e.target.value
      .split("")
      .map((s) => (s === "." ? undefined : s));

    if (value > 1 || digits.length > 8) {
      e.target.value = "1";
    }

    setEditedModelData({
      ...editedModelData,
      default_temperature:
        e.target.value === "" ? null : Math.min(parseFloat(e.target.value), 1),
    });
  };

  if (!editedModelData) return null;

  return (
    <>
      <FormField
        label="Default Temperature"
        defaultValue={editedModelData.default_temperature?.toString() ?? ""}
        placeholder="Default temperature"
        type="number"
        max="1"
        onChange={handleTemperatureChange}
      />

      <Flex direction="column" gap="2">
        <Text as="div" size="2" weight="bold">
          Capabilities
        </Text>
        <Flex gap="2" wrap="wrap">
          <CapabilityBadge
            name="Tools"
            enabled={editedModelData.supports_tools}
            onClick={() => toggleCapability("supports_tools")}
          />
          <CapabilityBadge
            name="Multimodality"
            enabled={editedModelData.supports_multimodality}
            onClick={() => toggleCapability("supports_multimodality")}
          />
          <CapabilityBadge
            name="Clicks"
            enabled={editedModelData.supports_clicks}
            onClick={() => toggleCapability("supports_clicks")}
          />
          <CapabilityBadge
            name="Agent"
            enabled={editedModelData.supports_agent}
            onClick={() => toggleCapability("supports_agent")}
          />
          <CapabilityBadge
            name="Reasoning"
            enabled={!!editedModelData.supports_reasoning}
            interactive={false}
          />

          {editedModelData.supports_reasoning && (
            <CapabilityBadge
              name="Boost Reasoning"
              enabled={!!editedModelData.supports_boost_reasoning}
              onClick={() => toggleCapability("supports_boost_reasoning")}
            />
          )}
        </Flex>
      </Flex>
    </>
  );
};

// Embedding model specific fields
type EmbeddingModelFieldsProps = {
  editedModelData: EmbeddingModel;
  setEditedModelDataByField: (field: string, value: string) => void;
};

const EmbeddingModelFields: FC<EmbeddingModelFieldsProps> = ({
  editedModelData,
  setEditedModelDataByField,
}) => {
  const handleFieldValueChange = (
    e: ChangeEvent<HTMLInputElement>,
    field: string,
  ) => {
    setEditedModelDataByField(field, e.target.value);
  };

  return (
    <>
      <FormField
        label="Embedding Size"
        defaultValue={editedModelData.embedding_size.toString()}
        onChange={(e) => handleFieldValueChange(e, "embedding_size")}
        placeholder="Embedding size"
        type="number"
      />
      <FormField
        label="Rejection Threshold"
        defaultValue={editedModelData.rejection_threshold.toString()}
        onChange={(e) => handleFieldValueChange(e, "rejection_threshold")}
        placeholder="Rejection threshold"
        type="number"
      />
      <FormField
        label="Embedding Batch"
        defaultValue={editedModelData.embedding_batch.toString()}
        onChange={(e) => handleFieldValueChange(e, "embedding_batch")}
        placeholder="Embedding batch"
        type="number"
      />
    </>
  );
};
