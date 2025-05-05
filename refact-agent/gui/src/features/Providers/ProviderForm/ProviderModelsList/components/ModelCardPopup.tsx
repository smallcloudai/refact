import type { FC } from "react";
import React, {
  useState,
  useEffect,
  useCallback,
  ChangeEvent,
  useMemo,
} from "react";
import isEqual from "lodash.isequal";
import { Button, Dialog, Flex, Text } from "@radix-ui/themes";

import {
  useGetCompletionModelFamiliesQuery,
  useGetModelConfiguration,
  useGetModelDefaults,
} from "../../../../../hooks/useModelsQuery";

import { FormField } from "./FormField";
import { CapabilityBadge } from "./CapabilityBadge";

import type {
  CodeChatModel,
  CodeCompletionModel,
  EmbeddingModel,
  Model,
  ModelType,
  SimplifiedModel,
  SupportsReasoningStyle,
} from "../../../../../services/refact";

import { extractHumanReadableReasoningType } from "../utils";
import { useEffectOnce } from "../../../../../hooks";
import { FormSelect } from "./FormSelect";
import { Spinner } from "../../../../../components/Spinner";

const SUPPORTED_REASONING_STYLES: SupportsReasoningStyle[] = [
  "openai",
  "deepseek",
  "anthropic",
  null,
];

export type ModelCardPopupProps = {
  minifiedModel?: SimplifiedModel;
  isOpen: boolean;
  isSaving: boolean;
  setIsOpen: (state: boolean) => void;
  onSave: (model: Model) => Promise<boolean>;
  onUpdate: ({
    model,
    oldModel,
  }: {
    model: Model;
    oldModel: SimplifiedModel;
  }) => Promise<boolean>;
  modelName: string;
  modelType: ModelType;
  providerName: string;
  currentModelNames: string[];
  newModelCreation?: boolean;
  isRemovable?: boolean;
};

export const ModelCardPopup: FC<ModelCardPopupProps> = ({
  isOpen,
  isSaving,
  setIsOpen,
  onSave,
  onUpdate,
  modelName,
  modelType,
  providerName,
  minifiedModel,
  currentModelNames,
  newModelCreation = false,
  isRemovable = false,
}) => {
  const {
    data: configuredModelData,
    isSuccess: _isConfiguredModelDataLoaded,
    currentData: configuredModelCurrentData,
  } = useGetModelConfiguration({
    modelName,
    modelType,
    providerName,
  });

  const { data: defaultModelData, isSuccess: isDefaultModelDataLoaded } =
    useGetModelDefaults({
      modelType,
      providerName,
    });
  const [editedModelData, setEditedModelData] = useState<Model | undefined>(
    configuredModelData,
  );

  const areDefaultsUnavailable = useMemo(() => {
    const dataToCompare = {
      ...editedModelData,
      name: "",
    };
    return isEqual(defaultModelData, dataToCompare);
  }, [defaultModelData, editedModelData]);

  const isSavingDisabled = useMemo(() => {
    if (!editedModelData?.name) {
      return true;
    }
    const isNameTaken = currentModelNames.some(
      (existingName) =>
        existingName === editedModelData.name && existingName !== modelName,
    );
    // TODO: maybe we should move it out somewhere :P
    const REQUIRED_FIELD_KEYS = ["tokenizer", "n_ctx"];

    const someFieldsNotFilled = Object.entries(editedModelData).some(
      ([key, value]) => {
        if (REQUIRED_FIELD_KEYS.includes(key)) {
          if (!value) return true;
        }

        return false;
      },
    );

    if (isNameTaken) return true;

    return isEqual(configuredModelData, editedModelData) || someFieldsNotFilled;
  }, [configuredModelData, editedModelData, currentModelNames, modelName]);

  useEffect(() => {
    if (isOpen) {
      if (configuredModelData) {
        setEditedModelData((prev) => {
          if (isEqual(prev, configuredModelCurrentData)) return prev;
          return configuredModelData;
        });
        return;
      }
      setEditedModelData(defaultModelData);
    }
  }, [
    isOpen,
    configuredModelData,
    configuredModelCurrentData,
    defaultModelData,
    newModelCreation,
    modelType,
  ]);

  useEffectOnce(() => {
    return () => {
      setEditedModelData(undefined);
    };
  });

  const handleSetDefaultModelData = useCallback(() => {
    if (!isDefaultModelDataLoaded) return;
    const updatedData = {
      ...defaultModelData,
      name: newModelCreation ? defaultModelData.name : modelName,
    };
    setEditedModelData(updatedData);
  }, [isDefaultModelDataLoaded, newModelCreation, modelName, defaultModelData]);

  const handleSave = useCallback(async () => {
    if (!isOpen || !editedModelData) return;

    let isSuccess: boolean;

    if (minifiedModel && minifiedModel.name !== editedModelData.name) {
      isSuccess = await onUpdate({
        model: editedModelData,
        oldModel: minifiedModel,
      });
    } else {
      isSuccess = await onSave(editedModelData);
    }
    if (!isSuccess) return;

    setTimeout(() => setIsOpen(false), 0);
  }, [isOpen, editedModelData, minifiedModel, setIsOpen, onSave, onUpdate]);

  const handleCancel = useCallback(() => {
    setTimeout(() => setIsOpen(false), 0);
  }, [setIsOpen]);

  const handleDialogChange = useCallback(
    (open: boolean) => {
      setIsOpen(open);
    },
    [setIsOpen],
  );

  const getValueByType = (value: string, valueType: string) => {
    if (valueType === "string") return value;
    if (valueType === "number") return parseFloat(value);
    return value;
  };

  const updateFieldByKey = useCallback(
    (key: string, value: string | number) => {
      if (!editedModelData) return;
      setEditedModelData({
        ...editedModelData,
        [key]: value,
      });
    },
    [editedModelData],
  );

  const handleFieldValueChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>, field: string) => {
      const valueType = typeof editedModelData?.[field as keyof Model];
      const value = getValueByType(e.target.value, valueType);
      updateFieldByKey(field, value);
    },
    [editedModelData, updateFieldByKey],
  );

  // Toggle capability value
  const toggleCapability = (key: string) => {
    if (!editedModelData) return;

    setEditedModelData({
      ...editedModelData,
      [key]: !editedModelData[key as keyof typeof editedModelData],
    });
  };

  if (!configuredModelData && !newModelCreation) {
    return null;
  }

  if (!configuredModelData && !newModelCreation) return null;

  return (
    <Dialog.Root open={isOpen} onOpenChange={handleDialogChange}>
      <Dialog.Content maxWidth="450px">
        <Dialog.Title>Model Configuration</Dialog.Title>
        <Dialog.Description size="2" mb="4">
          {!newModelCreation
            ? `Make changes to ${modelName} (${modelType} model)`
            : `Setup new model for ${providerName} (${modelType} model)`}
        </Dialog.Description>

        <Flex direction="column" gap="3">
          <FormField
            label="Name"
            value={editedModelData?.name}
            onChange={(e) => handleFieldValueChange(e, "name")}
            placeholder="Model name"
            isDisabled={!newModelCreation && !isRemovable}
          />
          {editedModelData?.type === "completion" && (
            <CompletionModelFields
              editedModelData={editedModelData}
              handleFieldValueChange={handleFieldValueChange}
              updateFieldByKey={updateFieldByKey}
            />
          )}

          {editedModelData?.type === "chat" && (
            <ChatModelFields
              editedModelData={editedModelData}
              handleFieldValueChange={handleFieldValueChange}
              setEditedModelData={setEditedModelData}
              toggleCapability={toggleCapability}
            />
          )}

          {editedModelData?.type === "embedding" && (
            <EmbeddingModelFields
              editedModelData={editedModelData}
              handleFieldValueChange={handleFieldValueChange}
            />
          )}
        </Flex>

        <Flex align="center" mt="4" justify="between" width="100%">
          <Flex gap="3" justify="end">
            <Button variant="soft" color="gray" onClick={handleCancel}>
              Cancel
            </Button>
            <Button
              disabled={isSaving || isSavingDisabled}
              onClick={() => void handleSave()}
            >
              {isSaving ? "Saving..." : "Save"}
            </Button>
          </Flex>
          <Button
            variant="outline"
            color="gray"
            onClick={handleSetDefaultModelData}
            title={
              areDefaultsUnavailable
                ? "Your configuration matches default one"
                : "Use model defaults"
            }
            disabled={areDefaultsUnavailable}
          >
            Use model defaults
          </Button>
        </Flex>
      </Dialog.Content>
    </Dialog.Root>
  );
};

type CompletionModelFieldsProps = {
  editedModelData: CodeCompletionModel;
  handleFieldValueChange: (
    e: ChangeEvent<HTMLInputElement>,
    field: string,
  ) => void;
  updateFieldByKey: (key: string, value: string | number) => void;
};

const CompletionModelFields: FC<CompletionModelFieldsProps> = ({
  editedModelData,
  handleFieldValueChange,
  updateFieldByKey,
}) => {
  const {
    data: modelFamiliesData,
    isSuccess,
    isLoading,
  } = useGetCompletionModelFamiliesQuery();
  if (isLoading || !isSuccess) return <Spinner spinning />;

  const aggregatedModelFamilies = [...modelFamiliesData.model_families, null];
  return (
    <>
      <FormField
        label="Context Window (n_ctx)"
        value={editedModelData.n_ctx.toString()}
        onChange={(e) => handleFieldValueChange(e, "n_ctx")}
        placeholder="Context window size"
        type="number"
      />
      <FormSelect
        label="Model Family"
        placeholder="Desired model family"
        value={editedModelData.model_family ?? "null"}
        onValueChange={(value) => updateFieldByKey("model_family", value)}
        options={aggregatedModelFamilies}
      />
    </>
  );
};

// Chat model specific fields
type ChatModelFieldsProps = {
  editedModelData?: CodeChatModel;
  setEditedModelData: (data: Model) => void;
  toggleCapability: (key: string) => void;
  handleFieldValueChange: (
    e: ChangeEvent<HTMLInputElement>,
    field: string,
  ) => void;
};

const ChatModelFields: FC<ChatModelFieldsProps> = ({
  editedModelData,
  setEditedModelData,
  toggleCapability,
  handleFieldValueChange,
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
      type: "chat",
      default_temperature:
        e.target.value === "" ? null : Math.min(parseFloat(e.target.value), 1),
    });
  };

  const handleReasoningStyleChange = (value: string) => {
    if (!editedModelData) return;

    setEditedModelData({
      ...editedModelData,
      type: "chat",
      supports_boost_reasoning:
        value === "null" ? false : editedModelData.supports_boost_reasoning,
      supports_reasoning:
        value === "null" ? null : (value as SupportsReasoningStyle),
    });
  };

  if (!editedModelData) return null;

  return (
    <>
      <FormField
        label="Context Window (n_ctx)"
        value={editedModelData.n_ctx.toString()}
        onChange={(e) => handleFieldValueChange(e, "n_ctx")}
        placeholder="Context window size"
        type="number"
      />
      <FormField
        label="Tokenizer"
        description="'hf://' stands for 'https://huggingface.co/'"
        value={editedModelData.tokenizer}
        onChange={(e) => handleFieldValueChange(e, "tokenizer")}
        placeholder="Tokenizer name"
      />
      <FormField
        label="Default Temperature"
        value={editedModelData.default_temperature?.toString() ?? ""}
        placeholder="Default temperature"
        type="number"
        max="1"
        onChange={handleTemperatureChange}
      />

      <Flex direction="column" gap="2">
        <FormSelect
          label="Reasoning Style"
          value={editedModelData.supports_reasoning ?? "null"}
          onValueChange={handleReasoningStyleChange}
          options={SUPPORTED_REASONING_STYLES}
          optionTransformer={extractHumanReadableReasoningType}
        />
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
  handleFieldValueChange: (
    e: ChangeEvent<HTMLInputElement>,
    field: string,
  ) => void;
};

const EmbeddingModelFields: FC<EmbeddingModelFieldsProps> = ({
  editedModelData,
  handleFieldValueChange,
}) => {
  return (
    <>
      <FormField
        label="Embedding Size"
        value={editedModelData.embedding_size.toString()}
        onChange={(e) => handleFieldValueChange(e, "embedding_size")}
        placeholder="Embedding size"
        type="number"
      />
      <FormField
        label="Rejection Threshold"
        value={editedModelData.rejection_threshold.toString()}
        onChange={(e) => handleFieldValueChange(e, "rejection_threshold")}
        placeholder="Rejection threshold"
        type="number"
      />
      <FormField
        label="Embedding Batch"
        value={editedModelData.embedding_batch.toString()}
        onChange={(e) => handleFieldValueChange(e, "embedding_batch")}
        placeholder="Embedding batch"
        type="number"
      />
    </>
  );
};
