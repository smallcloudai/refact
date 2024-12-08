import { type FormEvent, type FC, useState, ChangeEventHandler } from "react";
import { NotConfiguredIntegrationWithIconRecord } from "../../../services/refact";
import {
  Button,
  Card,
  Flex,
  Heading,
  RadioGroup,
  Text,
} from "@radix-ui/themes";
import { iconMap } from "../icons/iconMap";
import styles from "./IntegrationCmdline.module.css";
import { toPascalCase } from "../../../utils/toPascalCase";
import { formatProjectName } from "../../../utils/formatProjectName";
import { CustomInputField } from "../CustomFieldsAndWidgets";
import { Link } from "../../Link";

const validateSnakeCase = (value: string) => {
  const snakeCaseRegex = /^[a-z]+(_[a-z]+)*$/;
  return snakeCaseRegex.test(value);
};

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

const CMDLINE_TOOLS = ["cmdline", "service"];

export const IntegrationCmdline: FC<IntegrationCmdlineProps> = ({
  integration,
  handleSubmit,
}) => {
  const [integrationType, integrationTemplate] =
    integration.integr_name.split("_");
  const isIntegrationAComamndLine = CMDLINE_TOOLS.includes(integrationType);
  const [commandName, setCommandName] = useState("");
  const [errorMessage, setErrorMessage] = useState("");

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
      <Heading as="h3" size="4">
        <Flex align="center" gap="3">
          <img
            src={
              iconMap[isIntegrationAComamndLine ? "cmdline" : integrationType]
            }
            className={styles.integrationIcon}
          />
          {isIntegrationAComamndLine
            ? "Command Line Tool"
            : toPascalCase(integrationType)}
        </Flex>
      </Heading>
      <Text size="2" color="gray">
        Please, choose where you want to setup your integration
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
                      label: !shouldPathBeFormatted ? "Global" : path,
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
                  Please, write a name of your command in the text field below,
                  make sure that it&apos;s written in{" "}
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
                  ? "Please, fix all issues with the data"
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
