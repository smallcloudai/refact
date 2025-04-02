import React, { useEffect, useState } from "react";
import type { Provider, SimplifiedProvider } from "../../../services/refact";
import { Button, Flex, Select, TextField } from "@radix-ui/themes";
import { useGetProviderQuery } from "../../../hooks/useProvidersQuery";
import { Spinner } from "../../../components/Spinner";
import { toPascalCase } from "../../../utils/toPascalCase";
import isEqual from "lodash.isequal";
import { aggregateProviderFields } from "./utils";

import styles from "./ProviderForm.module.css";
import classNames from "classnames";

export type ProviderFormProps = {
  currentProvider: SimplifiedProvider<"name" | "enabled" | "readonly">;
  handleDiscardChanges: () => void;
  handleSaveChanges: (updatedProviderData: Provider) => void;
};

export const ProviderForm: React.FC<ProviderFormProps> = ({
  currentProvider,
  handleDiscardChanges,
  handleSaveChanges,
}) => {
  const { data: fullProviderData, isSuccess } = useGetProviderQuery({
    providerName: currentProvider.name,
  });

  const [formValues, setFormValues] = useState<Provider | null>(null);
  const [areShowingExtraFields, setAreShowingExtraFields] = useState(false);

  useEffect(() => {
    if (fullProviderData) {
      setFormValues(fullProviderData);
    }
  }, [fullProviderData]);

  const handleValuesChange = (updatedProviderData: Provider) => {
    setFormValues(updatedProviderData);
  };

  if (!isSuccess || !formValues) return <Spinner spinning />;

  const { extraFields, importantFields } = aggregateProviderFields(formValues);

  return (
    <Flex direction="column" width="100%" gap="2">
      {renderProviderFields({
        providerData: formValues,
        fields: importantFields,
        handleValuesChange,
      })}

      {areShowingExtraFields &&
        renderProviderFields({
          providerData: formValues,
          fields: extraFields,
          handleValuesChange,
        })}
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
      <Flex gap="2" align="center">
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
          disabled={isEqual(formValues, fullProviderData)}
          title="Save Provider configuration"
          onClick={() => handleSaveChanges(formValues)}
        >
          Save
        </Button>
      </Flex>
    </Flex>
  );
};

// eslint-disable-next-line react-refresh/only-export-components
export function renderProviderFields({
  providerData,
  fields,
  handleValuesChange,
}: {
  providerData: Provider;
  fields: Record<string, string>;
  handleValuesChange: (updatedProviderData: Provider) => void;
}) {
  return Object.entries(fields).map(([key, value], idx) => {
    if (key === "name" || key === "enabled" || key === "readonly") return null;

    if (key === "endpoint_style") {
      const availableOptions: Provider["endpoint_style"][] = ["openai", "hf"];

      return (
        <Flex key={`${key}_${idx}`} direction="column">
          {toPascalCase(key)}
          <Select.Root
            defaultValue={value.toString()}
            onValueChange={(value: Provider["endpoint_style"]) =>
              handleValuesChange({ ...providerData, endpoint_style: value })
            }
          >
            <Select.Trigger />
            <Select.Content>
              {availableOptions.map((option) => (
                <Select.Item key={option} value={option}>
                  {option}
                </Select.Item>
              ))}
            </Select.Content>
          </Select.Root>
        </Flex>
      );
    }
    return (
      <Flex key={`${key}_${idx}`} direction="column" gap="1">
        <label htmlFor={key}>{toPascalCase(key)}</label>
        <TextField.Root
          id={key}
          value={value.toString()}
          onChange={(event) =>
            handleValuesChange({ ...providerData, [key]: event.target.value })
          }
        />
      </Flex>
    );
  });
}
