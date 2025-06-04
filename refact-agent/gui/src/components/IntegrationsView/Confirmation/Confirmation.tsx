import type { FC } from "react";
import { useMemo } from "react";
import {
  IntegrationFieldValue,
  SchemaToolConfirmation,
  ToolConfirmation,
} from "../../../services/refact";
import { Flex, Heading, Text } from "@radix-ui/themes";
import { ConfirmationTable } from "../IntegrationsTable/ConfirmationTable";
import isEqual from "lodash.isequal";

type ConfirmationProps = {
  confirmationByUser: ToolConfirmation | null;
  confirmationFromValues: ToolConfirmation | null;
  defaultConfirmationObject: SchemaToolConfirmation;
  onChange: (fieldKey: string, fieldValue: IntegrationFieldValue) => void;
};

export const Confirmation: FC<ConfirmationProps> = ({
  confirmationByUser,
  confirmationFromValues,
  defaultConfirmationObject,
  onChange,
}) => {
  const transformedDefaultConfirmationObject: ToolConfirmation = useMemo(
    () => ({
      ask_user: defaultConfirmationObject.ask_user_default,
      deny: defaultConfirmationObject.deny_default,
    }),
    [defaultConfirmationObject],
  );
  const shouldBeTakenDefaults = !confirmationFromValues;

  const confirmationObjectToRender = shouldBeTakenDefaults
    ? transformedDefaultConfirmationObject
    : isEqual(confirmationFromValues, confirmationByUser)
      ? confirmationByUser
      : confirmationFromValues;

  return (
    <Flex direction="column" width="100%" gap="4" mt="4">
      <Heading as="h4" size="3">
        Confirmation Rules
      </Heading>
      <Text as="p" size="2" color="gray">
        Some commands might have destructive effects, here you can set up a list
        of patterns such that if a command matches one, you&apos;ll see a
        confirmation request or the command will be blocked completely.
      </Text>
      <Flex direction="column" width="100%" gap="3">
        {confirmationObjectToRender &&
          Object.entries(confirmationObjectToRender).map(([key, values]) => (
            <ConfirmationTable
              key={key}
              initialData={values}
              tableName={key}
              onToolConfirmation={(tableName, data) => {
                // Update the nested confirmation field
                onChange("confirmation", {
                  ...confirmationObjectToRender,
                  [tableName]: data,
                });
              }}
            />
          ))}
      </Flex>
    </Flex>
  );
};
