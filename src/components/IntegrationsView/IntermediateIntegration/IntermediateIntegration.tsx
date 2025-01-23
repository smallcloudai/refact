import {
  type FormEvent,
  type FC,
  useState,
  ChangeEventHandler,
  useEffect,
} from "react";
import { NotConfiguredIntegrationWithIconRecord } from "../../../services/refact";
import { Button, Card, Flex, RadioGroup, Text } from "@radix-ui/themes";
import { formatProjectName } from "../../../utils/formatProjectName";
import { CustomInputField } from "../CustomFieldsAndWidgets";
import { Link } from "../../Link";
import { useGetIntegrationDataByPathQuery } from "../../../hooks/useGetIntegrationDataByPathQuery";
import { debugIntegrations } from "../../../debugConfig";
import { validateSnakeCase } from "../../../utils/validateSnakeCase";

type IntegrationCmdlineProps = {
  integration: NotConfiguredIntegrationWithIconRecord;
  handleSubmit: (event: FormEvent<HTMLFormElement>) => void;
};

const renderIntegrationCmdlineField = ({
  path,
  label,
  shouldBeFormatted,
}: {
  path: string;
  label?: string;
  shouldBeFormatted: boolean;
}) => {
  const formattedLabel = shouldBeFormatted
    ? formatProjectName({
        projectPath: path,
        isMarkdown: false,
        indexOfLastFolder: 4,
      })
    : label;
  return (
    <Flex gap="2">
      <RadioGroup.Item value={path} /> {formattedLabel}
    </Flex>
  );
};

export const IntermediateIntegration: FC<IntegrationCmdlineProps> = ({
  integration,
  handleSubmit,
}) => {
  const [integrationType, integrationTemplate] =
    integration.integr_name.split("_");
  const [commandName, setCommandName] = useState(
    integrationType === "cmdline" || integrationType === "service"
      ? integration.commandName
      : "",
  );
  const [errorMessage, setErrorMessage] = useState("");

  const { integration: relatedIntegration } = useGetIntegrationDataByPathQuery(
    integration.integr_config_path[0],
  );

  const handleCommandNameChange: ChangeEventHandler<HTMLInputElement> = (
    event,
  ) => {
    const value = event.target.value;
    setCommandName(value);
    if (!validateSnakeCase(value)) {
      setErrorMessage("The command name must be in snake case!");
    } else {
      setErrorMessage("");
    }
  };

  useEffect(() => {
    debugIntegrations(`[DEBUG]: integration (not configured): `, integration);
  }, [integration]);

  return (
    <Flex direction="column" gap="4" width="100%">
      {relatedIntegration.data?.integr_schema.description && (
        <Text size="2" color="gray" mb="2">
          {relatedIntegration.data.integr_schema.description}
        </Text>
      )}
      <Text size="2" color="gray">
        Where do you want to configure this integration? Any project that has
        version control can have its own integrations configured.
      </Text>
      <form onSubmit={handleSubmit} id={`form-${integration.integr_name}`}>
        <Flex gap="5" direction="column" width="100%">
          <Card>
            <RadioGroup.Root
              name="integr_config_path"
              defaultValue={integration.integr_config_path[0]}
            >
              {integration.integr_config_path.map((path, index) => {
                const shouldPathBeFormatted =
                  integration.project_path[index] !== "";
                return (
                  <Text as="label" size="2" key={path}>
                    {renderIntegrationCmdlineField({
                      path,
                      label: !shouldPathBeFormatted
                        ? "Global, available for all projects"
                        : path,
                      shouldBeFormatted: shouldPathBeFormatted,
                    })}
                  </Text>
                );
              })}
            </RadioGroup.Root>
          </Card>
          <Flex direction="column" gap="3">
            {integrationTemplate && (
              <Flex direction="column" gap="2">
                <Text size="2" color="gray">
                  Name for your new command, make sure that it&apos;s written in{" "}
                  <Link
                    href="https://en.wikipedia.org/wiki/Snake_case"
                    target="_blank"
                  >
                    snake case
                  </Link>
                </Text>
                <Flex direction="column" gap="1">
                  <CustomInputField
                    name="command_name"
                    placeholder="runserver_py"
                    value={commandName}
                    onChange={handleCommandNameChange}
                    color={errorMessage ? "red" : undefined}
                  />
                  {errorMessage && (
                    <Text color="red" size="1">
                      {errorMessage}
                    </Text>
                  )}
                </Flex>
              </Flex>
            )}
            <Button
              type="submit"
              variant="surface"
              color="green"
              disabled={
                integrationTemplate ? !!errorMessage || !commandName : false
              }
              title={
                !!errorMessage || !commandName
                  ? "Please, fill out all required fields first"
                  : "Continue setting up integration"
              }
            >
              Continue with setup
            </Button>
          </Flex>
        </Flex>
      </form>
    </Flex>
  );
};
