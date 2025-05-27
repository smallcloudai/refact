import {
  Integration,
  IntegrationField,
  IntegrationPrimitive,
  ToolParameterEntity,
} from "../../../services/refact";
import { IntegrationFormField } from "../../../features/Integrations";
import { Grid } from "@radix-ui/themes";

import styles from "./IntegrationForm.module.css";
import { FC } from "react";

type FormFieldsProps = {
  integration: Integration;
  importantFields: Record<
    string,
    IntegrationField<NonNullable<IntegrationPrimitive>>
  >;
  extraFields: Record<
    string,
    IntegrationField<NonNullable<IntegrationPrimitive>>
  >;
  areExtraFieldsRevealed: boolean;
  onToolParameters: (data: ToolParameterEntity[]) => void;
  onArguments: (updatedArgs: string[]) => void;
  onEnvs: (updatedEnvs: Record<string, string>) => void;
  onHeaders: (updatedHeaders: Record<string, string>) => void;
};

export const FormFields: FC<FormFieldsProps> = ({
  integration,
  importantFields,
  extraFields,
  areExtraFieldsRevealed,
  onToolParameters,
  onArguments,
  onEnvs,
  onHeaders,
}) => {
  const {
    integr_config_path,
    integr_name,
    integr_schema,
    integr_values,
    project_path,
  } = integration;
  return (
    <Grid gap="2" className={styles.gridContainer}>
      {Object.keys(importantFields).map((fieldKey) => (
        <IntegrationFormField
          key={`${fieldKey}-important`}
          fieldKey={fieldKey}
          values={integr_values}
          field={integr_schema.fields[fieldKey]}
          integrationName={integr_name}
          integrationPath={integr_config_path}
          integrationProject={project_path}
          onToolParameters={onToolParameters}
          onArguments={onArguments}
          onEnvs={onEnvs}
          onHeaders={onHeaders}
        />
      ))}
      {Object.keys(extraFields).map((fieldKey) => (
        <IntegrationFormField
          key={`${fieldKey}-extra`}
          fieldKey={fieldKey}
          values={integr_values}
          field={integr_schema.fields[fieldKey]}
          integrationName={integr_name}
          integrationPath={integr_config_path}
          integrationProject={project_path}
          isFieldVisible={areExtraFieldsRevealed}
          onToolParameters={onToolParameters}
          onArguments={onArguments}
          onEnvs={onEnvs}
          onHeaders={onHeaders}
        />
      ))}
    </Grid>
  );
};
