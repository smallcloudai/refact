import {
  CustomBoolField,
  CustomDescriptionField,
  CustomInputField,
  CustomLabel,
} from "../../components/IntegrationsView/CustomFieldsAndWidgets";
import type {
  Integration,
  IntegrationField,
  IntegrationPrimitive,
} from "../../services/refact";
import styles from "./renderIntegrationFormField.module.css";
import { Box, Flex } from "@radix-ui/themes";
import { toPascalCase } from "../../utils/toPascalCase";
import { SmartLink } from "../../components/SmartLink";
// import { Markdown } from "../../components/Markdown";
import { ParametersTable } from "../../components/IntegrationsView/IntegrationsTable/ParametersTable";
import type { ToolParameterEntity } from "../../services/refact";
import { Markdown } from "../../components/Markdown";

type FieldType = "string" | "bool" | "int" | "tool" | "output";

const isFieldType = (value: string): value is FieldType => {
  return ["string", "bool", "int", "tool", "output"].includes(value);
};

const getDefaultValue = ({
  field,
  values,
  fieldKey,
  f_type,
}: {
  fieldKey: string;
  values: Integration["integr_values"];
  field: IntegrationField<NonNullable<IntegrationPrimitive>>;
  f_type: FieldType;
}) => {
  if (values && fieldKey in values) {
    return values[fieldKey]?.toString(); // Use the value from 'values' if present
  }

  if (f_type === "int") {
    return Number(field.f_default);
  }

  if (f_type === "bool") {
    return Boolean(field.f_default);
  }

  if (f_type === "tool") {
    // TODO: special logic for this type
    return JSON.stringify(field.f_default);
  }

  if (f_type === "output") {
    // TODO: special logic for this type
    return JSON.stringify(field.f_default);
  }

  return field.f_default?.toString(); // Otherwise, use the default value from the schema
};

export const renderIntegrationFormField = ({
  field,
  values,
  fieldKey,
  integrationName,
  integrationPath,
  isFieldVisible = true,
  integrationProject,
  onToolParameters,
}: {
  fieldKey: string;
  values: Integration["integr_values"];
  field: IntegrationField<NonNullable<IntegrationPrimitive>>;
  integrationName: string;
  integrationPath: string;
  integrationProject: string;
  isFieldVisible?: boolean;
  onToolParameters: (data: ToolParameterEntity[]) => void;
}) => {
  const [f_type_raw, f_size] = field.f_type.toString().split("_");
  const f_type = isFieldType(f_type_raw) ? f_type_raw : "string";

  const commonProps = {
    id: fieldKey,
    name: fieldKey,
    defaultValue: getDefaultValue({ field, fieldKey, values, f_type }),
    placeholder: field.f_placeholder?.toString(),
  };

  const maybeSmartlinks = field.smartlinks;

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
          label={field.f_label ? field.f_label : toPascalCase(fieldKey)}
          mt="2"
        />
      </Flex>
      <Flex direction="column" gap="2" align="start" width="100%">
        {f_type !== "bool" && f_type !== "output" && f_type !== "tool" && (
          <CustomInputField
            {...commonProps}
            type={f_type === "int" ? "number" : "text"}
            size={f_size}
            defaultValue={commonProps.defaultValue?.toString()}
          />
        )}
        {f_type === "bool" && (
          <CustomBoolField
            {...commonProps}
            defaultValue={Boolean(
              field.f_default
                ? field.f_default
                : values
                  ? values[fieldKey]
                  : "",
            )}
          />
        )}
        {f_type === "tool" && (
          <ParametersTable
            initialData={
              values ? (values[fieldKey] as ToolParameterEntity[]) : []
            }
            onToolParameters={onToolParameters}
          />
        )}
        {/* TODO: make output filter rendering better */}
        {f_type === "output" && (
          <Box>
            <Markdown>
              {"```json\n" +
                JSON.stringify(values ? values[fieldKey] : {}, null, 2) +
                "\n```"}
            </Markdown>
          </Box>
        )}
        {field.f_desc && (
          <CustomDescriptionField>{field.f_desc}</CustomDescriptionField>
        )}
        {/* TODO: implement EDITOR goto, and remove this condition */}
        {maybeSmartlinks &&
          !maybeSmartlinks.every(
            (smartlink) => smartlink.sl_goto?.startsWith("EDITOR"),
          ) && (
            <Flex align="center">
              {maybeSmartlinks.map((smartlink, index) => (
                <SmartLink
                  isSmall
                  key={`smartlink-${fieldKey}-${index}`}
                  smartlink={smartlink}
                  integrationName={integrationName}
                  integrationPath={integrationPath}
                  integrationProject={integrationProject}
                />
              ))}
            </Flex>
          )}
      </Flex>
    </Flex>
  );
};
