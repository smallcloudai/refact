import { Button, Flex, Grid, Text } from "@radix-ui/themes";
import classNames from "classnames";
import { FC, FormEvent, useEffect } from "react";
import { useGetIntegrationDataByPathQuery } from "../../../hooks/useGetIntegrationDataByPathQuery";
import { Spinner } from "../../Spinner";
import { Confirmation } from "../Confirmation";
import { useFormFields } from "../hooks/useFormFields";
import { IntegrationDocker } from "../IntegrationDocker";

import {
  areToolConfirmation,
  IntegrationFieldValue,
  type Integration,
} from "../../../services/refact";
import { ErrorState } from "./ErrorState";
import { FormAvailabilityAndDelete } from "./FormAvailabilityAndDelete";
import { FormFields } from "./FormFields";
import { FormSmartlinks } from "./FormSmartlinks";
import styles from "./IntegrationForm.module.css";
import { MCPLogs } from "./MCPLogs";
import { toPascalCase } from "../../../utils/toPascalCase";

type IntegrationFormProps = {
  integrationPath: string;
  isApplying: boolean;
  isDisabled: boolean;
  isDeletingIntegration: boolean;
  handleSubmit: (event: FormEvent<HTMLFormElement>) => void;
  handleDeleteIntegration: (path: string) => void;
  onSchema: (schema: Integration["integr_schema"]) => void;
  onValues: (values: Integration["integr_values"]) => void;
  handleSwitchIntegration: (
    integrationName: string,
    integrationConfigPath: string,
  ) => void;
  handleUpdateFormField: (
    fieldKey: string,
    fieldValue: IntegrationFieldValue,
  ) => void;
  formValues: Integration["integr_values"];
};

export const IntegrationForm: FC<IntegrationFormProps> = ({
  integrationPath,
  isApplying,
  isDisabled,
  isDeletingIntegration,
  handleSubmit,
  handleDeleteIntegration,
  onSchema,
  onValues,
  handleSwitchIntegration,
  handleUpdateFormField,
  formValues,
}) => {
  const { integration } = useGetIntegrationDataByPathQuery(integrationPath);

  const {
    importantFields,
    extraFields,
    areExtraFieldsRevealed,
    toggleExtraFields,
  } = useFormFields(integration.data?.integr_schema.fields);

  const schema = integration.data?.integr_schema;
  const values = integration.data?.integr_values;

  useEffect(() => {
    if (schema) {
      onSchema(schema);
    }
  }, [schema, onSchema]);

  useEffect(() => {
    if (values) {
      onValues(values);
    }
  }, [values, onValues]);

  if (integration.isLoading) {
    return <Spinner spinning />;
  }

  if (!integration.data) {
    return <Text>No integration found</Text>;
  }

  if (integration.data.error_log.length > 0) {
    return (
      <ErrorState
        integration={integration.data}
        onDelete={handleDeleteIntegration}
        isApplying={isApplying}
        isDeletingIntegration={isDeletingIntegration}
      />
    );
  }

  return (
    <Flex width="100%" direction="column" gap="2" pb="8">
      {integration.data.integr_schema.description && (
        <Text size="2" color="gray" mb="3">
          {integration.data.integr_schema.description}
        </Text>
      )}

      <form onSubmit={handleSubmit} id={`form-${integration.data.integr_name}`}>
        <Flex direction="column" gap="2">
          <Grid mb="0">
            <FormAvailabilityAndDelete
              integration={integration.data}
              isApplying={isApplying}
              isDeletingIntegration={isDeletingIntegration}
              formValues={formValues}
              onDelete={handleDeleteIntegration}
              onChange={handleUpdateFormField}
            />
            <FormSmartlinks
              integration={integration.data}
              smartlinks={integration.data.integr_schema.smartlinks}
            />
            <FormFields
              integration={integration.data}
              importantFields={importantFields}
              extraFields={extraFields}
              areExtraFieldsRevealed={areExtraFieldsRevealed}
              values={formValues}
              onChange={handleUpdateFormField}
            />
          </Grid>

          {Object.keys(extraFields).length > 0 && (
            <Button
              variant="soft"
              type="button"
              color="gray"
              size="2"
              onClick={toggleExtraFields}
              mb="1"
              mt={{ initial: "3", xs: "0" }}
              className={styles.advancedButton}
            >
              {areExtraFieldsRevealed
                ? "Hide advanced configuration"
                : "Show advanced configuration"}
            </Button>
          )}

          {!integration.data.integr_schema.confirmation.not_applicable && (
            <Flex gap="4" mb="3">
              <Confirmation
                confirmationByUser={
                  areToolConfirmation(formValues?.confirmation)
                    ? formValues.confirmation
                    : null
                }
                confirmationFromValues={
                  areToolConfirmation(
                    integration.data.integr_values?.confirmation,
                  )
                    ? integration.data.integr_values.confirmation
                    : null
                }
                defaultConfirmationObject={
                  integration.data.integr_schema.confirmation
                }
                onChange={handleUpdateFormField}
              />
            </Flex>
          )}

          <Flex
            justify="end"
            width="100%"
            position="fixed"
            bottom="4"
            right="8"
          >
            <Flex gap="4">
              <Button
                color="green"
                variant="solid"
                type="submit"
                size="2"
                title={isDisabled ? "Cannot apply, no changes made" : "Apply"}
                className={classNames(styles.button, styles.applyButton, {
                  [styles.disabledButton]: isApplying || isDisabled,
                })}
                disabled={isDisabled || isApplying}
              >
                {isApplying ? "Applying..." : "Apply"}
              </Button>
            </Flex>
          </Flex>
        </Flex>
      </form>

      {integration.data.integr_values !== null &&
        integration.data.integr_name.includes("mcp") && (
          <MCPLogs
            integrationPath={integration.data.integr_config_path}
            integrationName={toPascalCase(integration.data.integr_name)}
          />
        )}

      {integration.data.integr_schema.docker && (
        <Flex mt="6" direction="column" align="start" gap="5">
          <IntegrationDocker
            dockerData={integration.data.integr_schema.docker}
            integrationName={integration.data.integr_name}
            integrationProject={integration.data.project_path}
            integrationPath={integration.data.integr_config_path}
            handleSwitchIntegration={handleSwitchIntegration}
          />
        </Flex>
      )}
    </Flex>
  );
};
