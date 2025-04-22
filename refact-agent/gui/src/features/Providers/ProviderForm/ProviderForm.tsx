import React from "react";
import classNames from "classnames";
import { Button, Flex, Separator, Switch } from "@radix-ui/themes";

import { FormFields } from "./FormFields";
import { Spinner } from "../../../components/Spinner";

import { useProviderForm } from "./useProviderForm";
import type { Provider, SimplifiedProvider } from "../../../services/refact";

import { toPascalCase } from "../../../utils/toPascalCase";
import { aggregateProviderFields } from "./utils";

import styles from "./ProviderForm.module.css";
import { ProviderModelsList } from "./ProviderModelsList/ProviderModelsList";

export type ProviderFormProps = {
  currentProvider: SimplifiedProvider<
    "name" | "enabled" | "readonly" | "supports_completion"
  >;
  isProviderConfigured: boolean;
  isSaving: boolean;
  handleDiscardChanges: () => void;
  handleSaveChanges: (updatedProviderData: Provider) => void;
};

export const ProviderForm: React.FC<ProviderFormProps> = ({
  currentProvider,
  isProviderConfigured,
  isSaving,
  handleDiscardChanges,
  handleSaveChanges,
}) => {
  const {
    areShowingExtraFields,
    formValues,
    handleFormValuesChange,
    isProviderLoadedSuccessfully,
    setAreShowingExtraFields,
    shouldSaveButtonBeDisabled,
  } = useProviderForm({ providerName: currentProvider.name });

  if (!isProviderLoadedSuccessfully || !formValues) return <Spinner spinning />;

  const { extraFields, importantFields } = aggregateProviderFields(formValues);

  return (
    <Flex
      direction="column"
      width="100%"
      height="100%"
      mt="2"
      justify="between"
    >
      <Flex direction="column" width="100%" gap="2">
        <Flex align="center" justify="between" gap="3" mb="2">
          <label htmlFor={"enabled"}>{toPascalCase("enabled")}</label>
          <Switch
            id={"enabled"}
            checked={Boolean(formValues.enabled)}
            value={formValues.enabled ? "on" : "off"}
            disabled={formValues.readonly}
            className={classNames({
              [styles.disabledSwitch]: formValues.readonly,
            })}
            onCheckedChange={(checked) =>
              handleFormValuesChange({ ...formValues, ["enabled"]: checked })
            }
          />
        </Flex>
        <Separator size="4" mb="2" />
        <Flex direction="column" gap="2">
          <FormFields
            providerData={formValues}
            fields={importantFields}
            onChange={handleFormValuesChange}
          />
        </Flex>

        {areShowingExtraFields && (
          <Flex direction="column" gap="2" mt="4">
            <FormFields
              providerData={formValues}
              fields={extraFields}
              onChange={handleFormValuesChange}
            />
          </Flex>
        )}
        <Flex my="2" align="center" justify="center">
          <Button
            className={classNames(styles.button, styles.extraButton)}
            variant="ghost"
            color="gray"
            onClick={() => setAreShowingExtraFields((prev) => !prev)}
          >
            {areShowingExtraFields ? "Hide" : "Show"} advanced fields
          </Button>
        </Flex>
        {isProviderConfigured && (
          <ProviderModelsList provider={currentProvider} />
        )}
      </Flex>
      <Flex gap="2" align="center" mt="4">
        <Button
          className={styles.button}
          variant="outline"
          onClick={handleDiscardChanges}
        >
          Cancel
        </Button>
        <Button
          className={styles.button}
          variant="solid"
          disabled={isSaving || shouldSaveButtonBeDisabled}
          title="Save Provider configuration"
          onClick={() => handleSaveChanges(formValues)}
        >
          {isSaving ? "Saving..." : "Save"}
        </Button>
      </Flex>
    </Flex>
  );
};
