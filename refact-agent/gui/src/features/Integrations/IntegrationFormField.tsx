import { Box, Flex } from "@radix-ui/themes";
import { FC } from "react";
import {
  CustomBoolField,
  CustomDescriptionField,
  CustomInputField,
  CustomLabel,
} from "../../components/IntegrationsView/CustomFieldsAndWidgets";
import { SmartLink } from "../../components/SmartLink";
import { ParametersTable } from "../../components/IntegrationsView/IntegrationsTable/ParametersTable";
import { Markdown } from "../../components/Markdown";
import { toPascalCase } from "../../utils/toPascalCase";
import styles from "./IntegrationFormField.module.css";

import {
  areToolParameters,
  isMCPArgumentsArray,
  isMCPEnvironmentsDict,
  type Integration,
  type IntegrationField,
  type IntegrationPrimitive,
  type SmartLink as TSmartLink,
  type ToolParameterEntity,
} from "../../services/refact";
import { ArgumentsTable } from "../../components/IntegrationsView/IntegrationsTable/ArgumentsTable";
import { EnvironmentVariablesTable } from "../../components/IntegrationsView/IntegrationsTable/EnvironmentVariablesTable";

type FieldType = "string" | "bool" | "int" | "tool" | "output";

// Helper functions
const isFieldType = (value: string): value is FieldType => {
  return ["string", "bool", "int", "tool", "output"].includes(value);
};

const getDefaultValue = ({
  field,
  values,
  fieldKey,
  f_type,
}: {
  field: IntegrationField<NonNullable<IntegrationPrimitive>>;
  values: Integration["integr_values"];
  fieldKey: string;
  f_type: FieldType;
}): string | number | boolean | undefined => {
  // First check if we have a value in the current values
  if (values && fieldKey in values) {
    return values[fieldKey]?.toString();
  }

  // Otherwise use the default value based on type
  switch (f_type) {
    case "int":
      return Number(field.f_default);
    case "bool":
      return Boolean(field.f_default);
    case "tool":
    case "output":
      return JSON.stringify(field.f_default);
    default:
      return field.f_default?.toString();
  }
};

// Component types
type IntegrationFormFieldProps = {
  field: IntegrationField<NonNullable<IntegrationPrimitive>>;
  values: Integration["integr_values"];
  fieldKey: string;
  integrationName: string;
  integrationPath: string;
  integrationProject: string;
  isFieldVisible?: boolean;
  onToolParameters: (data: ToolParameterEntity[]) => void;
  onArguments: (updatedArgs: string[]) => void;
  onEnvs: (updatedEnvs: Record<string, string>) => void;
};

type CommonFieldProps = {
  id: string;
  name: string;
  defaultValue?: string | number | boolean;
  placeholder?: string;
};

// Components
const FieldContent: FC<{
  f_type: FieldType;
  commonProps: CommonFieldProps;
  f_size?: string;
  values: Integration["integr_values"];
  fieldKey: string;
  onToolParameters: (data: ToolParameterEntity[]) => void;
  onArguments: (updatedArgs: string[]) => void;
  onEnvs: (updatedEnvs: Record<string, string>) => void;
}> = ({
  f_type,
  commonProps,
  f_size,
  values,
  fieldKey,
  onToolParameters,
  onArguments,
  onEnvs,
}) => {
  switch (f_type) {
    case "bool": {
      return (
        <CustomBoolField
          {...commonProps}
          defaultValue={Boolean(
            commonProps.defaultValue ?? values?.[fieldKey] ?? false,
          )}
        />
      );
    }
    case "tool": {
      const valuesForTable = values?.[fieldKey];
      if (values && areToolParameters(valuesForTable)) {
        return (
          <ParametersTable
            initialData={valuesForTable}
            onToolParameters={onToolParameters}
          />
        );
      }
      break;
    }
    case "output": {
      return (
        <Box>
          <Markdown>
            {"```json\n" +
              JSON.stringify(values ? values[fieldKey] : {}, null, 2) +
              "\n```"}
          </Markdown>
        </Box>
      );
    }
    case "string": {
      if (f_size === "array") {
        const valuesForTable = values?.[fieldKey];
        const tableData = isMCPArgumentsArray(valuesForTable)
          ? valuesForTable
          : [];

        return (
          <ArgumentsTable
            initialData={tableData}
            onMCPArguments={onArguments}
          />
        );
      }
      if (f_size === "to_string_map") {
        const valuesForTable = values?.[fieldKey];
        const tableData = isMCPEnvironmentsDict(valuesForTable)
          ? valuesForTable
          : {};

        return (
          <EnvironmentVariablesTable
            initialData={tableData}
            onMCPEnvironmentVariables={onEnvs}
          />
        );
      }
      return (
        <CustomInputField
          {...commonProps}
          type={"text"}
          size={f_size}
          defaultValue={commonProps.defaultValue?.toString()}
        />
      );
    }
    default: {
      return (
        <CustomInputField
          {...commonProps}
          type="number"
          size={f_size}
          defaultValue={commonProps.defaultValue?.toString()}
        />
      );
    }
  }
};

const SmartLinks: FC<{
  smartlinks: TSmartLink[] | undefined;
  integrationName: string;
  integrationPath: string;
  integrationProject: string;
}> = ({ smartlinks, integrationName, integrationPath, integrationProject }) => {
  if (
    !smartlinks ||
    smartlinks.every((link) => link.sl_goto?.startsWith("EDITOR"))
  ) {
    return null;
  }

  return (
    <Flex align="center">
      {smartlinks.map((smartlink, index) => (
        <SmartLink
          isSmall
          key={`smartlink-${index}`}
          smartlink={smartlink}
          integrationName={integrationName}
          integrationPath={integrationPath}
          integrationProject={integrationProject}
        />
      ))}
    </Flex>
  );
};

export const IntegrationFormField: FC<IntegrationFormFieldProps> = ({
  field,
  values,
  fieldKey,
  integrationName,
  integrationPath,
  integrationProject,
  isFieldVisible = true,
  onToolParameters,
  onArguments,
  onEnvs,
}) => {
  const splittedType = field.f_type.toString().split("_");
  const [f_type_raw, ...rest] = splittedType;
  const f_size = rest.join("_");
  const f_type = isFieldType(f_type_raw) ? f_type_raw : "string";

  const defaultValue = getDefaultValue({ field, values, fieldKey, f_type });

  const commonProps = {
    id: fieldKey,
    name: fieldKey,
    defaultValue,
    placeholder: field.f_placeholder?.toString(),
  };

  return (
    <Flex
      direction="column"
      key={fieldKey}
      style={{
        width: "100%",
        opacity: isFieldVisible ? 1 : 0,
        height: isFieldVisible ? "auto" : 0,
        visibility: isFieldVisible ? "visible" : "hidden",
        position: isFieldVisible ? "inherit" : "absolute",
        transition: "opacity 0.3s ease-in-out",
      }}
      className={styles.flexField}
    >
      <Flex direction="column" width="100%" mb="2" className={styles.flexLabel}>
        <CustomLabel
          htmlFor={fieldKey}
          label={field.f_label ?? toPascalCase(fieldKey)}
          mt="2"
        />
      </Flex>

      <Flex direction="column" gap="2" align="start" width="100%">
        <FieldContent
          f_type={f_type}
          commonProps={commonProps}
          f_size={f_size}
          values={values}
          fieldKey={fieldKey}
          onToolParameters={onToolParameters}
          onArguments={onArguments}
          onEnvs={onEnvs}
        />

        {field.f_desc && (
          <CustomDescriptionField>{field.f_desc}</CustomDescriptionField>
        )}

        <SmartLinks
          smartlinks={field.smartlinks}
          integrationName={integrationName}
          integrationPath={integrationPath}
          integrationProject={integrationProject}
        />
      </Flex>
    </Flex>
  );
};
