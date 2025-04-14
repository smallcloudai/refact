import { FC } from "react";
import classNames from "classnames";

import { Flex, Select, TextField } from "@radix-ui/themes";
import { toPascalCase } from "../../../utils/toPascalCase";

import type { Provider } from "../../../services/refact";

import styles from "./ProviderForm.module.css";

export type FormFieldsProps = {
  providerData: Provider;
  fields: Record<string, string | boolean>;
  onChange: (updatedProviderData: Provider) => void;
};

export const FormFields: FC<FormFieldsProps> = ({
  providerData,
  fields,
  onChange,
}) => {
  return Object.entries(fields).map(([key, value], idx) => {
    if (key === "name" || key === "readonly" || key === "enabled") return null;

    if (key === "endpoint_style") {
      const availableOptions: Provider["endpoint_style"][] = ["openai", "hf"];

      return (
        <Flex key={`${key}_${idx}`} direction="column">
          {toPascalCase(key)}
          <Select.Root
            defaultValue={value.toString()}
            onValueChange={(value: Provider["endpoint_style"]) =>
              onChange({ ...providerData, endpoint_style: value })
            }
            disabled={providerData.readonly}
          >
            <Select.Trigger />
            <Select.Content position="popper">
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
            onChange({ ...providerData, [key]: event.target.value })
          }
          className={classNames({
            [styles.disabledField]: providerData.readonly,
          })}
          disabled={providerData.readonly}
        />
      </Flex>
    );
  });
};
