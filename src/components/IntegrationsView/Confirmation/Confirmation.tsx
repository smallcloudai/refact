import type { FC } from "react";
import { ToolConfirmation } from "../../../services/refact";
import { Flex, Heading, Text } from "@radix-ui/themes";
import { ConfirmationTable } from "../IntegrationsTable/ConfirmationTable";

type ConfirmationProps = {
  confirmationObject: ToolConfirmation;
  onChange: (fieldName: string, values: string[]) => void;
};

export const Confirmation: FC<ConfirmationProps> = ({
  confirmationObject,
  onChange,
}) => {
  return (
    <Flex direction="column" width="100%" gap="4">
      <Heading as="h4" size="4">
        Confirmation Rules
      </Heading>
      <Text>
        Some commands might have destructive effects, here you can set up a list
        of patterns such that if a command matches one, you&apos;ll see a
        confirmation request or the command will be blocked completely.
      </Text>
      {Object.entries(confirmationObject).map(([key, values]) => (
        <ConfirmationTable
          key={key}
          initialData={values}
          tableName={key}
          onToolConfirmation={onChange}
        />
      ))}
    </Flex>
  );
};
