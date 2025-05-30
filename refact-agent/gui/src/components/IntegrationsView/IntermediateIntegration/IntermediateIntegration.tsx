import {
  type FormEvent,
  type FC,
  useState,
  ChangeEventHandler,
  useMemo,
} from "react";
import { NotConfiguredIntegrationWithIconRecord } from "../../../services/refact";
import { Button, Card, Flex, RadioGroup, Text } from "@radix-ui/themes";
import { CustomInputField } from "../CustomFieldsAndWidgets";
import { Link } from "../../Link";
import { useGetIntegrationDataByPathQuery } from "../../../hooks/useGetIntegrationDataByPathQuery";
import { validateSnakeCase } from "../../../utils/validateSnakeCase";
import { createProjectLabelsWithConflictMarkers } from "../../../utils/createProjectLabelsWithConflictMarkers";
import { IntegrationPathField } from "./IntegrationPathField";

type IntegrationCmdlineProps = {
  integration: NotConfiguredIntegrationWithIconRecord;
  handleSubmit: (event: FormEvent<HTMLFormElement>) => void;
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

  const projectLabels = useMemo(() => {
    const validProjectPaths = integration.project_path.filter(
      (path) => path !== "",
    );
    return createProjectLabelsWithConflictMarkers(validProjectPaths); // Start with just the last folder name
  }, [integration.project_path]);

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
              {integration.integr_config_path.map((configPath, index) => {
                const shouldPathBeFormatted =
                  integration.project_path[index] !== "";

                return (
                  <Text as="label" size="2" key={configPath}>
                    <IntegrationPathField
                      configPath={configPath}
                      projectPath={integration.project_path[index]}
                      projectLabels={projectLabels}
                      shouldBeFormatted={shouldPathBeFormatted}
                    />
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
