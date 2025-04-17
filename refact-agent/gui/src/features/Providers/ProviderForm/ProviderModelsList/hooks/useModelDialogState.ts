import { useState, useCallback } from "react";

import {
  useDeleteModelMutation,
  useGetLazyModelConfiguration,
  useUpdateModelMutation,
} from "../../../../../hooks/useModelsQuery";
import { useAppDispatch } from "../../../../../hooks";

import { setInformation } from "../../../../Errors/informationSlice";
import { setError } from "../../../../Errors/errorsSlice";

import { modelsApi } from "../../../../../services/refact";
import type {
  Model,
  ModelType,
  SimplifiedModel,
} from "../../../../../services/refact";

/**
 * Custom hook for managing model dialog state with body style reset functionality
 */
export const useModelDialogState = ({
  modelType,
  providerName,
  initialState = false,
}: {
  modelType: ModelType;
  providerName: string;
  initialState?: boolean;
}) => {
  const dispatch = useAppDispatch();

  const [isOpen, setIsOpenState] = useState(initialState);
  const [isSavingModel, setIsSavingModel] = useState(false);
  const [isRemovingModel, setIsRemovingModel] = useState(false);
  const [dropdownOpen, setDropdownOpen] = useState(false);

  const getModelData = useGetLazyModelConfiguration();
  const updateModel = useUpdateModelMutation();
  const deleteModel = useDeleteModelMutation();

  const resetBodyStyles = useCallback(() => {
    document.body.style.pointerEvents = "";
  }, []);

  const setIsOpen = useCallback(
    (state: boolean) => {
      setIsOpenState(state);
      if (!state) {
        resetBodyStyles();
      }
    },
    [resetBodyStyles],
  );

  const openDialogSafely = useCallback(() => {
    setDropdownOpen(false);
    // Using a small timeout to avoid style conflicts
    setTimeout(() => {
      setIsOpenState(true);
    }, 10);
  }, []);

  const handleToggleModelEnabledState = useCallback(
    async (model: SimplifiedModel) => {
      setIsSavingModel(true);
      const { data: modelData } = await getModelData({
        providerName,
        modelName: model.name,
        modelType: modelType,
      });

      if (!modelData) {
        setIsSavingModel(false);
        return;
      }

      const enabled = modelData.enabled;

      const response = await updateModel({
        model: {
          ...modelData,
          enabled: !enabled,
        },
        provider: providerName,
        type: modelType,
      });

      if (response.error) {
        dispatch(
          setError(
            `Error occurred on ${enabled ? "disabling" : "enabling"} ${
              model.name
            } configuration. Check if your model configuration is correct`,
          ),
        );
        setIsSavingModel(false);
        return;
      }

      const actions = [
        setInformation(
          `Model ${model.name} ${
            enabled ? "disabled" : "enabled"
          } successfully!`,
        ),
        modelsApi.util.invalidateTags(["MODELS", "MODEL"]),
      ];

      actions.forEach((action) => dispatch(action));
      setIsSavingModel(false);
    },
    [dispatch, getModelData, updateModel, modelType, providerName],
  );

  const handleRemoveModel = useCallback(
    async (
      model: SimplifiedModel,
      operationType: "remove" | "reset" = "remove",
    ) => {
      const response = await deleteModel({
        model: model.name,
        provider: providerName,
        type: modelType,
      });

      if (response.error) {
        dispatch(
          setError(
            `Something went wrong during ${
              operationType === "remove" ? "removal" : "reset"
            } of ${model.name} model. Please, try again`,
          ),
        );
        setIsRemovingModel(false);
        return false;
      }

      const actions = [
        setInformation(
          `Model ${model.name} was ${
            operationType === "remove" ? "removed" : "reset"
          } successfully!`,
        ),
        modelsApi.util.invalidateTags(["MODELS", "MODEL"]),
      ];

      actions.forEach((action) => dispatch(action));
      setIsRemovingModel(false);
      return true;
    },
    [dispatch, deleteModel, providerName, modelType],
  );

  const handleResetModel = useCallback(
    async (model: SimplifiedModel) => {
      const isSuccess = await handleRemoveModel(model, "reset");
      if (isSuccess) {
        dispatch(modelsApi.util.invalidateTags(["MODELS", "MODEL"]));
      }
    },
    [dispatch, handleRemoveModel],
  );

  const handleSaveModel = useCallback(
    async (modelData: Model) => {
      setIsSavingModel(true);
      const response = await updateModel({
        model: modelData,
        provider: providerName,
        type: modelType,
      });

      if (response.error) {
        dispatch(
          setError(
            `Something went wrong during update of ${modelData.name} model. Please, try again`,
          ),
        );
        setIsSavingModel(false);
        return false;
      }
      const actions = [
        setInformation(`Model ${modelData.name} was updated successfully!`),
        modelsApi.util.invalidateTags(["MODELS", "MODEL"]),
      ];

      actions.forEach((action) => dispatch(action));
      setIsSavingModel(false);
      return true;
    },
    [dispatch, setIsSavingModel, providerName, modelType, updateModel],
  );

  return {
    isOpen,
    isSavingModel,
    isRemovingModel,
    setIsRemovingModel,
    setIsSavingModel,
    setIsOpen,
    dropdownOpen,
    setDropdownOpen,
    openDialogSafely,
    resetBodyStyles,
    handleSaveModel,
    handleRemoveModel,
    handleResetModel,
    handleToggleModelEnabledState,
  };
};
