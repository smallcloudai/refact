import React, { useCallback, useEffect, useState } from "react";
import classNames from "classnames";
import { useGetIntegrationDataByPathQuery } from "../../../hooks/useGetIntegrationDataByPathQuery";

import type { FC, FormEvent, Dispatch } from "react";
import type {
  Integration,
  IntegrationField,
  IntegrationPrimitive,
} from "../../../services/refact";

import styles from "./IntegrationForm.module.css";
import { Spinner } from "../../Spinner";
import { Button, Flex, Grid, Heading, Text } from "@radix-ui/themes";
import { IntegrationDocker } from "../IntegrationDocker";
import { SmartLink } from "../../SmartLink";
import { renderIntegrationFormField } from "../../../features/Integrations/renderIntegrationFormField";
import { IntegrationAvailability } from "./IntegrationAvailability";
import { toPascalCase } from "../../../utils/toPascalCase";
import { iconMap } from "../icons/iconMap";
import { IntegrationDeletePopover } from "../IntegrationDeletePopover";
import { debugIntegrations } from "../../../debugConfig";
import { selectThemeMode } from "../../../features/Config/configSlice";
import { useAppSelector } from "../../../hooks";

function areAllFieldsBoolean(json: unknown): json is Record<string, boolean> {
  return (
    typeof json === "object" &&
    json !== null &&
    Object.values(json).every((value) => typeof value === "boolean")
  );
}

type IntegrationFormProps = {
  integrationPath: string;
  isApplying: boolean;
  isDisabled: boolean;
  isDeletingIntegration: boolean;
  availabilityValues: Record<string, boolean>;
  handleSubmit: (event: FormEvent<HTMLFormElement>) => void;
  handleDeleteIntegration: (path: string, name: string) => void;
  handleChange: (event: FormEvent<HTMLFormElement>) => void;
  onSchema: (schema: Integration["integr_schema"]) => void;
  onValues: (values: Integration["integr_values"]) => void;
  setAvailabilityValues: Dispatch<
    React.SetStateAction<Record<string, boolean>>
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
  handleSubmit,
  handleDeleteIntegration,
  handleChange,
  onSchema,
  onValues,
  setAvailabilityValues,
  handleSwitchIntegration,
}) => {
  const [areExtraFieldsRevealed, setAreExtraFieldsRevealed] = useState(false);

  const { integration } = useGetIntegrationDataByPathQuery(integrationPath);

  const theme = useAppSelector(selectThemeMode);
  const icons = iconMap(
    theme ? (theme === "inherit" ? "light" : theme) : "light",
  );

  const handleAvailabilityChange = useCallback(
    (fieldName: string, value: boolean) => {
      setAvailabilityValues((prev) => ({ ...prev, [fieldName]: value }));
    },
    [setAvailabilityValues],
  );

  useEffect(() => {
    if (
      integration.data?.integr_values.available &&
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

  return (
    <Flex width="100%" direction="column" gap="2">
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
          <Grid mt="2" mb="0">
            <Flex gap="4" mb="3" className={styles.switchInline}>
              {integration.data.integr_values.available &&
                Object.entries(integration.data.integr_values.available).map(
                  ([key, _]: [string, boolean]) => (
                    <IntegrationAvailability
                      key={key}
                      fieldName={key}
                      value={availabilityValues[key]}
                      onChange={handleAvailabilityChange}
                    />
                  ),
                )}
            </Flex>
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
              mt="3"
              className={styles.advancedButton}
            >
              {areExtraFieldsRevealed
                ? "Hide advanced configuration"
                : "Show advanced configuration"}
            </Button>
          )}
          <Flex justify="end" width="100%">
            <Flex gap="4">
              <IntegrationDeletePopover
                integrationName={integration.data.integr_name}
                integrationConfigPath={integration.data.integr_config_path}
                isApplying={isApplying}
                isDeletingIntegration={isDeletingIntegration}
                handleDeleteIntegration={handleDeleteIntegration}
              />
              <Button
                color="green"
                variant="solid"
                type="submit"
                size="2"
                title={isDisabled ? "Cannot apply, no changes made" : "Apply"}
                className={classNames(
                  { [styles.disabledButton]: isApplying || isDisabled },
                  styles.button,
                  styles.applyButton,
                )}
                disabled={isDisabled}
              >
                {isApplying ? "Applying..." : "Apply"}
              </Button>
            </Flex>
          </Flex>
        </Flex>
      </form>
      {integration.data.integr_schema.smartlinks &&
        integration.data.integr_schema.smartlinks.length > 0 && (
          <Flex width="100%" direction="column" gap="2" mt="4">
            <Heading as="h4" size="3">
              Ask AI to do it for you (experimental)
            </Heading>
            <Flex align="center" gap="2" mt="2" wrap="wrap">
              {integration.data.integr_schema.smartlinks.map(
                (smartlink, index) => {
                  return (
                    <SmartLink
                      key={`smartlink-${index}`}
                      smartlink={smartlink}
                      integrationName={integration.data?.integr_name ?? ""}
                      integrationProject={integration.data?.project_path ?? ""}
                      integrationPath={
                        integration.data?.integr_config_path ?? ""
                      }
                    />
                  );
                },
              )}
            </Flex>
          </Flex>
        )}
      {integration.data.integr_schema.docker && (
        <Flex mt="6" direction="column" align="start" gap="5">
          <Flex gap="2" align="center" justify="center" width="100%">
            <img
              src={icons.docker}
              className={styles.DockerIcon}
              alt={integration.data.integr_name}
            />
            <Heading as="h3" align="left">
              {toPascalCase(integration.data.integr_name)} Containers
            </Heading>
          </Flex>
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
