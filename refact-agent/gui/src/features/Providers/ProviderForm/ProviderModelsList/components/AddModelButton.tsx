import type { FC } from "react";
import { useModelDialogState } from "../hooks";
import { ModelType } from "../../../../../services/refact";
import { ModelCardPopup } from "./ModelCardPopup";
import { Button } from "@radix-ui/themes";

export type AddModelButtonProps = {
  modelType: ModelType;
  providerName: string;
  currentModelNames: string[];
};

export const AddModelButton: FC<AddModelButtonProps> = ({
  modelType,
  providerName,
  currentModelNames,
}) => {
  const {
    isOpen,
    setIsOpen,
    isSavingModel,
    handleSaveModel,
    handleUpdateModel,
  } = useModelDialogState({
    modelType,
    providerName,
    initialState: false,
  });

  return (
    <>
      <ModelCardPopup
        isOpen={isOpen}
        isSaving={isSavingModel}
        setIsOpen={setIsOpen}
        providerName={providerName}
        modelName=""
        modelType={modelType}
        onSave={handleSaveModel}
        onUpdate={handleUpdateModel}
        currentModelNames={currentModelNames}
        newModelCreation
      />
      <Button
        variant="outline"
        size="1"
        color="gray"
        onClick={() => setIsOpen(!isOpen)}
      >
        Add model
      </Button>
    </>
  );
};
