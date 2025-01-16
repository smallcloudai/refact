import React, { useCallback, useEffect, useMemo, useState } from "react";
import classNames from "classnames";
import { useGetIntegrationDataByPathQuery } from "../../../hooks/useGetIntegrationDataByPathQuery";

import type { FC, FormEvent, Dispatch } from "react";
import type {
  Integration,
  IntegrationField,
  IntegrationPrimitive,
  ToolConfirmation,
} from "../../../services/refact";

import styles from "./IntegrationForm.module.css";
import { Spinner } from "../../Spinner";
import { Badge, Button, Flex, Grid, Heading, Text } from "@radix-ui/themes";
import { IntegrationDocker } from "../IntegrationDocker";
import { SmartLink } from "../../SmartLink";
import { renderIntegrationFormField } from "../../../features/Integrations/renderIntegrationFormField";
import { IntegrationAvailability } from "./IntegrationAvailability";
import { IntegrationDeletePopover } from "../IntegrationDeletePopover";
import { debugIntegrations } from "../../../debugConfig";
import type { ToolParameterEntity } from "../../../services/refact";
import {
  areAllFieldsBoolean,
  areToolConfirmation,
  areToolParameters,
} from "../../../services/refact";
import { Confirmation } from "../Confirmation";
import { ExclamationTriangleIcon } from "@radix-ui/react-icons";
import { useEventsBusForIDE } from "../../../hooks";

type IntegrationFormProps = {
  integrationPath: string;
  isApplying: boolean;
  isDisabled: boolean;
  isDeletingIntegration: boolean;
  availabilityValues: Record<string, boolean>;
  confirmationRules: ToolConfirmation;
  handleSubmit: (event: FormEvent<HTMLFormElement>) => void;
  handleDeleteIntegration: (path: string, name: string) => void;
  handleChange: (event: FormEvent<HTMLFormElement>) => void;
  onSchema: (schema: Integration["integr_schema"]) => void;
  onValues: (values: Integration["integr_values"]) => void;
  setAvailabilityValues: Dispatch<
    React.SetStateAction<Record<string, boolean>>
  >;
  setConfirmationRules: Dispatch<React.SetStateAction<ToolConfirmation>>;
  setToolParameters: Dispatch<
    React.SetStateAction<ToolParameterEntity[] | null>
  >;
  handleSwitchIntegration: (
    integrationName: string,
    integrationConfigPath: string,
  ) => void;
};

export const IntegrationForm: FC<IntegrationFormProps> = ({
  integrationPath,
  isApplying,
  isDisabled,
  isDeletingIntegration,
  availabilityValues,
  confirmationRules,
  handleSubmit,
  handleDeleteIntegration,
  handleChange,
  onSchema,
  onValues,
  setAvailabilityValues,
  setConfirmationRules,
  setToolParameters,
  handleSwitchIntegration,
}) => {
  const [areExtraFieldsRevealed, setAreExtraFieldsRevealed] = useState(false);

  const { integration } = useGetIntegrationDataByPathQuery(integrationPath);
  const { openFile } = useEventsBusForIDE();

  const handleAvailabilityChange = useCallback(
    (fieldName: string, value: boolean) => {
      setAvailabilityValues((prev) => ({ ...prev, [fieldName]: value }));
    },
    [setAvailabilityValues],
  );

  const handleConfirmationChange = useCallback(
    (fieldName: string, values: string[]) => {
      setConfirmationRules((prev) => {
        return { ...prev, [fieldName as keyof ToolConfirmation]: values };
      });
    },
    [setConfirmationRules],
  );

  const handleToolParameters = useCallback(
    (value: ToolParameterEntity[]) => {
      setToolParameters(value);
    },
    [setToolParameters],
  );

  const shouldIntegrationFormBeDisabled = useMemo(() => {
    if (!integration.data?.integr_values) return false;
    return isDisabled;
  }, [isDisabled, integration]);

  useEffect(() => {
    if (
      integration.data?.integr_values &&
      integration.data.integr_values.available &&
      typeof integration.data.integr_values.available === "object" &&
      areAllFieldsBoolean(integration.data.integr_values.available)
    ) {
      Object.entries(integration.data.integr_values.available).forEach(
        ([key, value]) => {
          handleAvailabilityChange(key, value);
        },
      );
    }
  }, [integration, handleAvailabilityChange]);

  useEffect(() => {
    if (
      integration.data?.integr_values &&
      integration.data.integr_values.parameters &&
      Array.isArray(integration.data.integr_values.parameters) &&
      areToolParameters(integration.data.integr_values.parameters)
    ) {
      handleToolParameters(integration.data.integr_values.parameters);
    }
  }, [integration, handleToolParameters]);

  useEffect(() => {
    if (integration.data?.integr_schema) {
      onSchema(integration.data.integr_schema);
    }

    if (integration.data?.integr_values) {
      onValues(integration.data.integr_values);
    }
    debugIntegrations(`[DEBUG]: integration.data: `, integration);
  }, [integration, onSchema, onValues]);

  const importantFields = Object.entries(
    integration.data?.integr_schema.fields ?? {},
  )
    .filter(([_, field]) => !field.f_extra)
    .reduce<
      Record<string, IntegrationField<NonNullable<IntegrationPrimitive>>>
    >((acc, [key, field]) => {
      acc[key] = field;
      return acc;
    }, {});

  const extraFields = Object.entries(
    integration.data?.integr_schema.fields ?? {},
  )
    .filter(([_, field]) => field.f_extra)
    .reduce<
      Record<string, IntegrationField<NonNullable<IntegrationPrimitive>>>
    >((acc, [key, field]) => {
      acc[key] = field;
      return acc;
    }, {});

  if (integration.isLoading) {
    return <Spinner spinning />;
  }

  if (!integration.data) {
    return (
      <div>
        <p>No integration found</p>
      </div>
    );
  }

  if (integration.data.error_log.length > 0) {
    const errorMessage = integration.data.error_log[0].error_msg;
    const integrationFile = integration.data.error_log[0].integr_config_path;
    const errorLine = integration.data.error_log[0].error_line;
    return (
      <Flex width="100%" direction="column" align="start" gap="4">
        <Text size="2" color="gray">
          Whoops, this integration has a syntax error in the config file. You
          can fix this problem by editing the config file.
        </Text>
        <Badge size="2" color="red">
          <ExclamationTriangleIcon />
          {errorMessage}
        </Badge>
        <Flex align="center" gap="2">
          <Button
            variant="outline"
            color="gray"
            onClick={() =>
              openFile({
                file_name: integrationFile,
                line: errorLine === 0 ? 1 : errorLine,
              })
            }
          >
            Open {integration.data.integr_name}.yaml
          </Button>
          <IntegrationDeletePopover
            integrationName={integration.data.integr_name}
            integrationConfigPath={integration.data.integr_config_path}
            isApplying={isApplying}
            isDeletingIntegration={isDeletingIntegration}
            handleDeleteIntegration={handleDeleteIntegration}
          />
        </Flex>
      </Flex>
    );
  }

  return (
    <Flex width="100%" direction="column" gap="2" pb="8">
      {integration.data.integr_schema.description && (
        <Text size="2" color="gray" mb="3">
          {integration.data.integr_schema.description}
        </Text>
      )}
      <form
        onSubmit={handleSubmit}
        onChange={handleChange}
        id={`form-${integration.data.integr_name}`}
      >
        <Flex direction="column" gap="2">
          <Grid mb="0">
            {integration.data.integr_values && (
              <Flex align="start" justify="between">
                <Flex
                  gap="4"
                  mb="4"
                  align="center"
                  justify="between"
                  className={styles.switchInline}
                >
                  {integration.data.integr_values.available &&
                    Object.keys(integration.data.integr_values.available).map(
                      (key) => (
                        <IntegrationAvailability
                          key={key}
                          fieldName={key}
                          value={availabilityValues[key]}
                          onChange={handleAvailabilityChange}
                        />
                      ),
                    )}
                </Flex>
                <IntegrationDeletePopover
                  integrationName={integration.data.integr_name}
                  integrationConfigPath={integration.data.integr_config_path}
                  isApplying={isApplying}
                  isDeletingIntegration={isDeletingIntegration}
                  handleDeleteIntegration={handleDeleteIntegration}
                />
              </Flex>
            )}
            {integration.data.integr_schema.smartlinks &&
              integration.data.integr_schema.smartlinks.length > 0 && (
                <Flex width="100%" direction="column" gap="1" mb="6">
                  <Flex align="center" gap="3" mt="2" wrap="wrap">
                    <Heading as="h6" size="2" weight="medium">
                      Actions:
                    </Heading>
                    {integration.data.integr_schema.smartlinks.map(
                      (smartlink, index) => {
                        return (
                          <SmartLink
                            key={`smartlink-${index}`}
                            smartlink={smartlink}
                            integrationName={
                              integration.data?.integr_name ?? ""
                            }
                            integrationProject={
                              integration.data?.project_path ?? ""
                            }
                            integrationPath={
                              integration.data?.integr_config_path ?? ""
                            }
                            shouldBeDisabled={
                              smartlink.sl_enable_only_with_tool
                                ? integration.data?.integr_values === null ||
                                  !shouldIntegrationFormBeDisabled ||
                                  !availabilityValues.on_your_laptop
                                : false
                            }
                          />
                        );
                      },
                    )}
                  </Flex>
                </Flex>
              )}
            <Grid gap="2" className={styles.gridContainer}>
              {Object.keys(importantFields).map((fieldKey) => {
                if (integration.data) {
                  return renderIntegrationFormField({
                    fieldKey: fieldKey,
                    values: integration.data.integr_values,
                    field: integration.data.integr_schema.fields[fieldKey],
                    integrationName: integration.data.integr_name,
                    integrationPath: integration.data.integr_config_path,
                    integrationProject: integration.data.project_path,
                    onToolParameters: handleToolParameters,
                  });
                }
              })}
              {Object.keys(extraFields).map((fieldKey) => {
                if (integration.data) {
                  return renderIntegrationFormField({
                    fieldKey: fieldKey,
                    values: integration.data.integr_values,
                    field: integration.data.integr_schema.fields[fieldKey],
                    integrationName: integration.data.integr_name,
                    integrationPath: integration.data.integr_config_path,
                    integrationProject: integration.data.project_path,
                    isFieldVisible: areExtraFieldsRevealed,
                    onToolParameters: handleToolParameters,
                  });
                }
              })}
            </Grid>
          </Grid>
          {Object.values(extraFields).length > 0 && (
            <Button
              variant="soft"
              type="button"
              color="gray"
              size="2"
              onClick={() => setAreExtraFieldsRevealed((prev) => !prev)}
              mb="1"
              mt={{
                initial: "3",
                xs: "0",
              }}
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
                confirmationByUser={confirmationRules}
                confirmationFromValues={
                  integration.data.integr_values !== null &&
                  areToolConfirmation(
                    integration.data.integr_values.confirmation,
                  )
                    ? integration.data.integr_values.confirmation
                    : null
                }
                defaultConfirmationObject={
                  integration.data.integr_schema.confirmation
                }
                onChange={handleConfirmationChange}
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
                title={
                  shouldIntegrationFormBeDisabled
                    ? "Cannot apply, no changes made"
                    : "Apply"
                }
                className={classNames(
                  {
                    [styles.disabledButton]:
                      isApplying || shouldIntegrationFormBeDisabled,
                  },
                  styles.button,
                  styles.applyButton,
                )}
                disabled={shouldIntegrationFormBeDisabled}
              >
                {isApplying ? "Applying..." : "Apply"}
              </Button>
            </Flex>
          </Flex>
        </Flex>
      </form>
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
