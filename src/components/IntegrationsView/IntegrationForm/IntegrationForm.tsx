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
import { Button, DataList, Flex, Heading } from "@radix-ui/themes";
import { IntegrationDocker } from "../IntegrationDocker";
import { SmartLink } from "../../SmartLink";
import { renderIntegrationFormField } from "../../../features/Integrations/renderIntegrationFormField";
import { IntegrationAvailability } from "./IntegrationAvailability";
import { toPascalCase } from "../../../utils/toPascalCase";
import { debugIntegrations } from "../../../debugConfig";
import { iconMap } from "../icons/iconMap";

type IntegrationFormProps = {
  integrationPath: string;
  isApplying: boolean;
  isDisabled: boolean;
  availabilityValues: Record<string, boolean>;
  handleSubmit: (event: FormEvent<HTMLFormElement>) => void;
  handleChange: (event: FormEvent<HTMLFormElement>) => void;
  onSchema: (schema: Integration["integr_schema"]) => void;
  onValues: (values: Integration["integr_values"]) => void;
  setAvailabilityValues: Dispatch<
    React.SetStateAction<Record<string, boolean>>
  >;
};

export const IntegrationForm: FC<IntegrationFormProps> = ({
  integrationPath,
  isApplying,
  isDisabled,
  availabilityValues,
  handleSubmit,
  handleChange,
  onSchema,
  onValues,
  setAvailabilityValues,
}) => {
  const [areExtraFieldsRevealed, setAreExtraFieldsRevealed] = useState(false);

  const { integration } = useGetIntegrationDataByPathQuery(integrationPath);

  const handleAvailabilityChange = useCallback(
    (fieldName: string, value: boolean) => {
      setAvailabilityValues((prev) => ({ ...prev, [fieldName]: value }));
    },
    [setAvailabilityValues],
  );

  useEffect(() => {
    if (
      integration.data?.integr_values.available &&
      typeof integration.data.integr_values.available === "object"
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
      <form
        onSubmit={handleSubmit}
        onChange={handleChange}
        id={`form-${integration.data.integr_name}`}
      >
        <Flex direction="column" gap="2">
          <DataList.Root
            mt="2"
            mb="0"
            size="1"
            orientation={{
              xs: "horizontal",
              initial: "vertical",
            }}
          >
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
            {Object.keys(importantFields).map((fieldKey) => {
              if (integration.data) {
                return renderIntegrationFormField({
                  fieldKey: fieldKey,
                  values: integration.data.integr_values,
                  field: integration.data.integr_schema.fields[fieldKey],
                  integrationName: integration.data.integr_name,
                  integrationPath: integration.data.project_path,
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
                  integrationPath: integration.data.project_path,
                  isFieldVisible: areExtraFieldsRevealed,
                });
              }
            })}
          </DataList.Root>
          {Object.values(extraFields).length > 0 && (
            <Button
              variant="soft"
              type="button"
              color="gray"
              size="2"
              onClick={() => setAreExtraFieldsRevealed((prev) => !prev)}
              mb="2"
            >
              {areExtraFieldsRevealed ? "Hide" : "Show more"}
            </Button>
          )}
          <Flex justify="between" width="100%">
            <Flex gap="4">
              <Button
                color="green"
                variant="solid"
                type="submit"
                size="2"
                title={isDisabled ? "Cannot apply, no changes made" : "Apply"}
                className={classNames(
                  { [styles.disabledButton]: isApplying || isDisabled },
                  styles.button,
                )}
                disabled={isDisabled}
              >
                {isApplying ? "Applying..." : "Apply"}
              </Button>
            </Flex>
            <Flex align="center" gap="4">
              {integration.data.integr_schema.smartlinks.map(
                (smartlink, index) => {
                  return (
                    <SmartLink
                      key={`smartlink-${index}`}
                      smartlink={smartlink}
                      integrationName={integration.data?.integr_name ?? ""}
                      integrationPath={integration.data?.project_path ?? ""}
                    />
                  );
                },
              )}
            </Flex>
          </Flex>
        </Flex>
      </form>
      {integration.data.integr_schema.docker && (
        <Flex mt="6" direction="column" align="start" gap="5">
          <Flex gap="2" align="center" justify="center" width="100%">
            <img
              src={iconMap.docker}
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
            integrationPath={integration.data.project_path}
          />
        </Flex>
      )}
    </Flex>
  );
};
