import type { FC } from "react";
import { useModelDialogState } from "../hooks";
import { ModelType } from "../../../../../services/refact";
import { ModelCardPopup } from "./ModelCardPopup";
import { Button } from "@radix-ui/themes";

export type AddModelButtonProps = {
  modelType: ModelType;
  providerName: string;
};

export const AddModelButton: FC<AddModelButtonProps> = ({
  modelType,
  providerName,
}) => {
  const { isOpen, setIsOpen, isSavingModel, handleSaveModel } =
    useModelDialogState({
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
