import type { FC } from "react";
import { ToolConfirmation } from "../../../services/refact";
import { Flex, Heading } from "@radix-ui/themes";
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
        Setup rules for your integration
      </Heading>
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
