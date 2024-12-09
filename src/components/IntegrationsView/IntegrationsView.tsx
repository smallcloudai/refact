import { Box, Flex, Heading, Text } from "@radix-ui/themes";
import { FetchBaseQueryError } from "@reduxjs/toolkit/query";
import type { FC, FormEvent } from "react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { debugIntegrations } from "../../debugConfig";
import {
  clearError,
  getErrorMessage,
  setError,
} from "../../features/Errors/errorsSlice";
import {
  clearInformation,
  getInformationMessage,
  setInformation,
} from "../../features/Errors/informationSlice";
import {
  isIntegrationSetupPage,
  pop,
  popBackTo,
  selectCurrentPage,
} from "../../features/Pages/pagesSlice";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { useSaveIntegrationData } from "../../hooks/useSaveIntegrationData";
import {
  dockerApi,
  Integration,
  integrationsApi,
  IntegrationWithIconRecord,
  IntegrationWithIconResponse,
  isDetailMessage,
  isNotConfiguredIntegrationWithIconRecord,
  isPrimitive,
  NotConfiguredIntegrationWithIconRecord,
} from "../../services/refact";
import { ErrorCallout } from "../Callout";
import { InformationCallout } from "../Callout/Callout";
import { Markdown } from "../Markdown";
import { Spinner } from "../Spinner";
import { IntegrationCard } from "./IntegrationCard";
import { IntegrationForm } from "./IntegrationForm";
import { IntegrationsHeader } from "./IntegrationsHeader";
import styles from "./IntegrationsView.module.css";
import { iconMap } from "./icons/iconMap";
import { LeftRightPadding } from "../../features/Integrations/Integrations";
import { IntermediateIntegration } from "./IntermediateIntegration";
import { parseOrElse } from "../../utils";

type IntegrationViewProps = {
  integrationsMap?: IntegrationWithIconResponse;
  leftRightPadding: LeftRightPadding;
  // integrationsIcons?: IntegrationIcon[];
  isLoading: boolean;
  goBack?: () => void;
  handleIfInnerIntegrationWasSet: (state: boolean) => void;
};

const INTEGRATIONS_WITH_TERMINAL_ICON = ["cmdline", "service"];

export const IntegrationsView: FC<IntegrationViewProps> = ({
  integrationsMap,
  isLoading,
  leftRightPadding,
  goBack,
  handleIfInnerIntegrationWasSet,
}) => {
  const dispatch = useAppDispatch();
  const globalError = useAppSelector(getErrorMessage);
  const information = useAppSelector(getInformationMessage);
  const { saveIntegrationMutationTrigger } = useSaveIntegrationData();
  // const currentThreadIntegration = useAppSelector(selectIntegration);
  const currentPage = useAppSelector(selectCurrentPage);
  const currentThreadIntegration = useMemo(() => {
    if (!currentPage) return null;
    if (!isIntegrationSetupPage(currentPage)) return null;
    return currentPage;
  }, [currentPage]);

  const maybeIntegration = useMemo(() => {
    if (!currentThreadIntegration) return null;
    if (!integrationsMap) return null;
    return (
      integrationsMap.integrations.find(
        (integration) =>
          integration.integr_config_path ===
          currentThreadIntegration.integrationPath,
      ) ?? null
    );
  }, [currentThreadIntegration, integrationsMap]);

  // TBD: what if they went home then came back to integrations?

  const [currentIntegration, setCurrentIntegration] =
    useState<IntegrationWithIconRecord | null>(maybeIntegration);

  const [currentNotConfiguredIntegration, setCurrentNotConfiguredIntegration] =
    useState<NotConfiguredIntegrationWithIconRecord | null>(null);

  useEffect(() => {
    if (maybeIntegration) {
      setCurrentIntegration(maybeIntegration);
    }
  }, [maybeIntegration]);

  const [currentIntegrationSchema, setCurrentIntegrationSchema] = useState<
    Integration["integr_schema"] | null
  >(null);

  const [currentIntegrationValues, setCurrentIntegrationValues] = useState<
    Integration["integr_values"] | null
  >(null);

  const [isApplyingIntegrationForm, setIsApplyingIntegrationForm] =
    useState<boolean>(false);

  const [isDisabledIntegrationForm, setIsDisabledIntegrationForm] =
    useState<boolean>(true);

  const [availabilityValues, setAvailabilityValues] = useState<
    Record<string, boolean>
  >({});

  useEffect(() => {
    debugIntegrations(`[DEBUG]: integrationsData: `, integrationsMap);
  }, [integrationsMap]);

  useEffect(() => {
    if (currentIntegration ?? currentNotConfiguredIntegration) {
      handleIfInnerIntegrationWasSet(true);
    } else {
      handleIfInnerIntegrationWasSet(false);
    }
  }, [
    currentIntegration,
    currentNotConfiguredIntegration,
    handleIfInnerIntegrationWasSet,
  ]);

  const globalIntegrations = useMemo(() => {
    if (integrationsMap?.integrations) {
      return integrationsMap.integrations.filter(
        (integration) =>
          integration.project_path === "" && integration.integr_config_exists,
      );
    }
  }, [integrationsMap]);

  const projectSpecificIntegrations = useMemo(() => {
    if (integrationsMap?.integrations) {
      return integrationsMap.integrations.filter(
        (integration) => integration.project_path !== "",
      );
    }
  }, [integrationsMap]);

  const groupedProjectIntegrations = useMemo(() => {
    if (projectSpecificIntegrations) {
      return projectSpecificIntegrations.reduce<
        Record<string, IntegrationWithIconResponse["integrations"]>
      >((acc, integration) => {
        if (integration.integr_config_exists) {
          if (!(integration.project_path in acc)) {
            acc[integration.project_path] = [];
          }
          acc[integration.project_path].push(integration);
        }
        return acc;
      }, {});
    }
  }, [projectSpecificIntegrations]);

  const nonConfiguredIntegrations = useMemo(() => {
    if (integrationsMap?.integrations) {
      const groupedIntegrations = integrationsMap.integrations.reduce<
        Record<string, NotConfiguredIntegrationWithIconRecord>
      >((acc, integration) => {
        if (!integration.integr_config_exists) {
          if (!(integration.integr_name in acc)) {
            acc[integration.integr_name] = {
              ...integration,
              project_path: [integration.project_path],
              integr_config_path: [integration.integr_config_path],
              integr_config_exists: false,
            };
          } else {
            if (
              !acc[integration.integr_name].project_path.includes(
                integration.project_path,
              )
            ) {
              acc[integration.integr_name].project_path.push(
                integration.project_path,
              );
            }
            acc[integration.integr_name].integr_config_path.push(
              integration.integr_config_path,
            );
          }
        }
        return acc;
      }, {});

      // Sort paths so that paths containing ".config" are first
      Object.values(groupedIntegrations).forEach((integration) => {
        integration.project_path.sort((a, _b) => (a === "" ? -1 : 1));
        integration.integr_config_path.sort((a, _b) =>
          a.includes(".config") ? -1 : 1,
        );
      });

      return Object.values(groupedIntegrations);
    }
  }, [integrationsMap]);

  useEffect(() => {
    debugIntegrations(
      `[DEBUG]: nonConfiguredIntegrations: `,
      nonConfiguredIntegrations,
    );
  }, [nonConfiguredIntegrations]);

  const handleSetCurrentIntegrationSchema = (
    schema: Integration["integr_schema"],
  ) => {
    if (!currentIntegration) return;

    setCurrentIntegrationSchema(schema);
  };

  const handleSetCurrentIntegrationValues = (
    values: Integration["integr_values"],
  ) => {
    if (!currentIntegration) return;

    setCurrentIntegrationValues(values);
  };

  const handleFormReturn = useCallback(() => {
    if (currentIntegration) {
      setCurrentIntegration(null);
      setIsDisabledIntegrationForm(true);
    }
    if (currentNotConfiguredIntegration) {
      setCurrentNotConfiguredIntegration(null);
      setIsDisabledIntegrationForm(true);
    }
    information && dispatch(clearInformation());
    globalError && dispatch(clearError());
    dispatch(integrationsApi.util.resetApiState());
    dispatch(dockerApi.util.resetApiState());
    // TODO: can cause a loop where integration pages goes back to form
    dispatch(pop());
    dispatch(popBackTo({ name: "integrations page" }));
  }, [
    dispatch,
    globalError,
    information,
    currentIntegration,
    currentNotConfiguredIntegration,
  ]);

  const handleSubmit = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      if (!currentIntegration) return;
      debugIntegrations(`[DEBUG]: schema: `, currentIntegrationSchema);
      if (!currentIntegrationSchema) return;
      event.preventDefault();
      setIsApplyingIntegrationForm(true);

      debugIntegrations(`[DEBUG]: event: `, event);

      const formData = new FormData(event.currentTarget);
      const rawFormValues = Object.fromEntries(formData.entries());
      debugIntegrations(`[DEBUG]: rawFormValues: `, rawFormValues);
      // Adjust types of data based on f_type of each field in schema
      const formValues: Integration["integr_values"] = Object.keys(
        rawFormValues,
      ).reduce<Integration["integr_values"]>((acc, key) => {
        const field = currentIntegrationSchema.fields[key];
        const [f_type, _f_size] = (field.f_type as string).split("_");
        switch (f_type) {
          case "int":
            acc[key] = parseInt(rawFormValues[key] as string, 10);
            break;
          case "string":
            acc[key] = rawFormValues[key] as string;
            break;
          case "bool":
            acc[key] = rawFormValues[key] === "on" ? true : false;
            break;
          case "tool":
            acc[key] = parseOrElse<Integration["integr_values"][number]>(
              rawFormValues[key] as string,
              {},
            );
            break;
          case "output":
            acc[key] = parseOrElse<Integration["integr_values"][number]>(
              rawFormValues[key] as string,
              {},
            );
            break;
          default:
            acc[key] = rawFormValues[key] as string;
            break;
        }
        return acc;
      }, {});

      debugIntegrations(`[DEBUG]: formValues: `, formValues);

      formValues.available = availabilityValues;

      const response = await saveIntegrationMutationTrigger(
        currentIntegration.integr_config_path,
        formValues,
      );

      if (response.error) {
        const error = response.error as FetchBaseQueryError;
        debugIntegrations(`[DEBUG]: error is present, error: `, error);
        dispatch(
          setError(
            isDetailMessage(error.data)
              ? error.data.detail
              : `something went wrong while saving configuration for ${currentIntegration.integr_name} integration`,
          ),
        );
      } else {
        dispatch(
          setInformation(
            `Integration ${currentIntegration.integr_name} saved successfully.`,
          ),
        );
        setIsDisabledIntegrationForm(true);
      }
      setIsApplyingIntegrationForm(false);
    },
    [
      currentIntegration,
      saveIntegrationMutationTrigger,
      currentIntegrationSchema,
      dispatch,
      availabilityValues,
    ],
  );

  const handleIntegrationFormChange = useCallback(
    (event: FormEvent<HTMLFormElement>) => {
      if (!currentIntegration) return;
      if (!currentIntegrationSchema) return;
      if (!currentIntegrationValues) return;
      if (!currentIntegrationValues.available) return;
      event.preventDefault();

      const formData = new FormData(event.currentTarget);
      const rawFormValues = Object.fromEntries(formData.entries());

      // Adjust types of data based on f_type of each field in schema
      const formValues: Integration["integr_values"] = Object.keys(
        rawFormValues,
      ).reduce<Integration["integr_values"]>((acc, key) => {
        const field = currentIntegrationSchema.fields[key];
        const [f_type, _f_size] = (field.f_type as string).split("_");
        switch (f_type) {
          case "int":
            acc[key] = parseInt(rawFormValues[key] as string, 10);
            break;
          case "string":
            acc[key] = rawFormValues[key] as string;
            break;
          case "bool":
            acc[key] = rawFormValues[key] === "on" ? true : false;
            break;
          case "tool":
            acc[key] = parseOrElse<Integration["integr_values"][number]>(
              rawFormValues[key] as string,
              {},
            );
            break;
          case "output":
            acc[key] = parseOrElse<Integration["integr_values"][number]>(
              rawFormValues[key] as string,
              {},
            );
            break;
          default:
            acc[key] = rawFormValues[key] as string;
            break;
        }
        return acc;
      }, {});

      const eachFormValueIsNotChanged = Object.entries(formValues).every(
        ([fieldKey, fieldValue]) => {
          if (isPrimitive(fieldValue)) {
            return (
              fieldKey in currentIntegrationValues &&
              fieldValue === currentIntegrationValues[fieldKey]
            );
          }
          // TODO: better comparison of objects?
          if (typeof fieldValue === "object") {
            return (
              fieldKey in currentIntegrationValues &&
              JSON.stringify(fieldValue) ===
                JSON.stringify(currentIntegrationValues[fieldKey])
            );
          }
        },
      );

      debugIntegrations(
        `[DEBUG]: eachFormValueIsNotChanged: `,
        eachFormValueIsNotChanged,
      );

      const eachAvailabilityOptionIsNotChanged = Object.entries(
        availabilityValues,
      ).every(([fieldKey, fieldValue]) => {
        const availableObj = currentIntegrationValues.available;
        if (availableObj && typeof availableObj === "object") {
          return (
            fieldKey in availableObj && fieldValue === availableObj[fieldKey]
          );
        }
        return false;
      });

      debugIntegrations(`[DEBUG]: formValues: `, formValues);
      debugIntegrations(
        `[DEBUG]: currentIntegrationValues: `,
        currentIntegrationValues,
      );
      debugIntegrations(
        `[DEBUG]: eachAvailabilityOptionIsNotChanged: `,
        eachAvailabilityOptionIsNotChanged,
      );
      debugIntegrations(`[DEBUG]: availabilityValues: `, availabilityValues);
      const maybeDisabled =
        eachFormValueIsNotChanged && eachAvailabilityOptionIsNotChanged;
      debugIntegrations(`[DEBUG CHANGE]: maybeDisabled: `, maybeDisabled);

      setIsDisabledIntegrationForm(maybeDisabled);
    },
    [
      currentIntegration,
      currentIntegrationValues,
      currentIntegrationSchema,
      availabilityValues,
    ],
  );

  const handleNotConfiguredIntegrationSubmit = useCallback(
    (event: FormEvent<HTMLFormElement>) => {
      if (!integrationsMap) return;
      if (!currentNotConfiguredIntegration) return;
      event.preventDefault();
      debugIntegrations(`[DEBUG]: event: `, event);
      const formData = new FormData(event.currentTarget);
      const rawFormValues = Object.fromEntries(formData.entries());
      debugIntegrations(`[DEBUG]: rawFormValues: `, rawFormValues);
      if (
        "integr_config_path" in rawFormValues &&
        typeof rawFormValues.integr_config_path === "string" &&
        "command_name" in rawFormValues &&
        typeof rawFormValues.command_name === "string"
      ) {
        // making integration-get call and setting the result as currentIntegration
        const commandName = rawFormValues.command_name;
        const configPath = rawFormValues.integr_config_path.replace(
          "TEMPLATE",
          commandName,
        );

        debugIntegrations(
          `[DEBUG]: config path for \`v1/integration-get\`: `,
          configPath,
        );

        const customIntegration: IntegrationWithIconRecord = {
          when_isolated: false,
          on_your_laptop: false,
          integr_name: `cmdline_${commandName}`,
          integr_config_path: configPath,
          project_path: rawFormValues.integr_config_path
            .toString()
            .includes(".config")
            ? ""
            : rawFormValues.integr_config_path.toString(),
          integr_config_exists: false,
        };

        setCurrentIntegration(customIntegration);
        setCurrentNotConfiguredIntegration(null);
        return;
      } else if ("integr_config_path" in rawFormValues) {
        // getting config path, opening integration
        const foundIntegration = integrationsMap.integrations.find(
          (integration) =>
            integration.integr_config_path === rawFormValues.integr_config_path,
        );
        if (!foundIntegration) {
          debugIntegrations(`[DEBUG]: integration was not found, error!`);
          return;
        }
        setCurrentIntegration(foundIntegration);
        setCurrentNotConfiguredIntegration(null);
      } else {
        debugIntegrations(
          `[DEBUG]: Unexpected error occured. It's mostly a bug`,
        );
      }
    },
    [currentNotConfiguredIntegration, integrationsMap],
  );

  const handleNavigateToIntegrationSetup = useCallback(
    (integrationName: string, integrationConfigPath: string) => {
      if (!integrationsMap) return;
      if (!currentIntegration) return;
      debugIntegrations(
        `[DEBUG]: integrationConfigPath: `,
        integrationConfigPath,
      );
      // TODO: this should be probably made not in hardcoded style, user needs to choose which docker he wants to setup
      const maybeIntegration = integrationsMap.integrations.find(
        (integration) =>
          integration.integr_name === integrationName &&
          integration.project_path === "",
      );
      if (!maybeIntegration) {
        debugIntegrations(
          `[DEBUG]: desired integration was not found in the list of all available ones :/`,
        );
        return;
      }
      setIsDisabledIntegrationForm(true);
      setCurrentIntegration(maybeIntegration);
    },
    [currentIntegration, integrationsMap],
  );

  const integrationLogo = useMemo(() => {
    if (!currentIntegration && !currentNotConfiguredIntegration) {
      return "https://placehold.jp/150x150.png";
    }
    return INTEGRATIONS_WITH_TERMINAL_ICON.includes(
      currentIntegration
        ? currentIntegration.integr_name.split("_")[0]
        : currentNotConfiguredIntegration
          ? currentNotConfiguredIntegration.integr_name.split("_")[0]
          : "https://placehold.jp/150x150.png",
    )
      ? iconMap.cmdline
      : iconMap[
          currentIntegration
            ? currentIntegration.integr_name
            : currentNotConfiguredIntegration
              ? currentNotConfiguredIntegration.integr_name
              : ""
        ];
  }, [currentIntegration, currentNotConfiguredIntegration]);

  if (isLoading) {
    return <Spinner spinning />;
  }

  const goBackAndClearError = () => {
    goBack && goBack();
    dispatch(clearError());
    setCurrentIntegration(null);
  };

  const handleIntegrationShowUp = (
    integration:
      | IntegrationWithIconRecord
      | NotConfiguredIntegrationWithIconRecord,
  ) => {
    if (isNotConfiguredIntegrationWithIconRecord(integration)) {
      handleNotSetupIntegrationShowUp(integration);
      return;
    }
    debugIntegrations(`[DEBUG]: open form: `, integration);
    setCurrentIntegration(integration);
  };
  const handleNotSetupIntegrationShowUp = (
    integration: NotConfiguredIntegrationWithIconRecord,
  ) => {
    if (!integrationsMap) return;

    debugIntegrations(
      `[DEBUG]: open form for not configured integration: `,
      integration,
    );

    setCurrentNotConfiguredIntegration(integration);
    // setCurrentIntegration(integration);
  };

  if (!integrationsMap) {
    return (
      <ErrorCallout
        className={styles.popup}
        mx="0"
        onClick={goBackAndClearError}
      >
        fetching integrations.
      </ErrorCallout>
    );
  }

  return (
    <Box
      style={{
        width: "inherit",
        height: "100%",
      }}
    >
      <Flex
        direction="column"
        style={{
          width: "100%",
          height: "100%",
        }}
      >
        {(currentIntegration ?? currentNotConfiguredIntegration) && (
          <IntegrationsHeader
            leftRightPadding={leftRightPadding}
            handleFormReturn={handleFormReturn}
            integrationName={
              currentIntegration
                ? currentIntegration.integr_name
                : currentNotConfiguredIntegration
                  ? currentNotConfiguredIntegration.integr_name
                  : ""
            }
            icon={integrationLogo}
          />
        )}
        {currentNotConfiguredIntegration && (
          <Flex
            direction="column"
            align="start"
            justify="between"
            height="100%"
          >
            <IntermediateIntegration
              handleSubmit={(event) =>
                handleNotConfiguredIntegrationSubmit(event)
              }
              integration={currentNotConfiguredIntegration}
            />
          </Flex>
        )}
        {currentIntegration && (
          <Flex
            direction="column"
            align="start"
            justify="between"
            height="100%"
          >
            <IntegrationForm
              // TODO: on smart link click or pass the name down
              handleSubmit={(event) => void handleSubmit(event)}
              integrationPath={currentIntegration.integr_config_path}
              isApplying={isApplyingIntegrationForm}
              isDisabled={isDisabledIntegrationForm}
              onSchema={handleSetCurrentIntegrationSchema}
              onValues={handleSetCurrentIntegrationValues}
              handleChange={handleIntegrationFormChange}
              availabilityValues={availabilityValues}
              setAvailabilityValues={setAvailabilityValues}
              handleSwitchIntegration={handleNavigateToIntegrationSetup}
            />
            {information && (
              <InformationCallout
                timeout={3000}
                mx="0"
                onClick={() => dispatch(clearInformation())}
                className={styles.popup}
              >
                {information}
              </InformationCallout>
            )}
            {globalError && (
              <ErrorCallout
                mx="0"
                timeout={3000}
                onClick={() => dispatch(clearError())}
                className={styles.popup}
              >
                {globalError}
              </ErrorCallout>
            )}
          </Flex>
        )}
        {!currentIntegration && !currentNotConfiguredIntegration && (
          <Flex direction="column" width="100%" gap="4">
            <Text my="2">
              Integrations allow Refact.ai Agent to interact with other services
              and tools
            </Text>
            <Flex
              align="start"
              direction="column"
              justify="between"
              gap="4"
              width="100%"
            >
              <Heading
                as="h4"
                size="3"
                style={{
                  width: "100%",
                }}
              >
                ⚙️ Globally configured{" "}
                {globalIntegrations ? globalIntegrations.length : 0}{" "}
                {globalIntegrations &&
                  (globalIntegrations.length > 1 ||
                  globalIntegrations.length === 0
                    ? "integrations"
                    : "integration")}
              </Heading>
              <Text size="2" color="gray">
                Global configurations are shared in your IDE and available for
                all your projects.
              </Text>
              {globalIntegrations && (
                <Flex direction="column" align="start" gap="3" width="100%">
                  {globalIntegrations.map((integration, index) => (
                    <IntegrationCard
                      key={`${index}-${integration.integr_config_path}`}
                      integration={integration}
                      handleIntegrationShowUp={handleIntegrationShowUp}
                    />
                  ))}
                </Flex>
              )}
            </Flex>
            {groupedProjectIntegrations &&
              Object.entries(groupedProjectIntegrations).map(
                ([projectPath, integrations], index) => {
                  debugIntegrations(projectPath);
                  const formattedProjectName =
                    "```.../" +
                    projectPath.split(/[/\\]/)[
                      projectPath.split(/[/\\]/).length - 1
                    ] +
                    "/```";

                  return (
                    <Flex
                      key={`project-group-${index}`}
                      direction="column"
                      gap="4"
                      align="start"
                    >
                      <Heading as="h4" size="3">
                        <Flex align="start" gap="3" justify="center">
                          ⚙️ In
                          <Markdown>{formattedProjectName}</Markdown>
                          configured {integrations.length}{" "}
                          {integrations.length > 1 || integrations.length === 0
                            ? "integrations"
                            : "integration"}
                        </Flex>
                      </Heading>
                      <Text size="2" color="gray">
                        Folder-specific integrations are local integrations,
                        which are shared only in folder-specific scope.
                      </Text>
                      <Flex
                        direction="column"
                        align="start"
                        gap="2"
                        width="100%"
                      >
                        {integrations.map((integration, subIndex) => (
                          <IntegrationCard
                            key={`project-${index}-${subIndex}-${integration.integr_config_path}`}
                            integration={integration}
                            handleIntegrationShowUp={handleIntegrationShowUp}
                          />
                        ))}
                      </Flex>
                    </Flex>
                  );
                },
              )}
            <Flex direction="column" gap="4" align="start">
              <Heading as="h4" size="3">
                <Flex align="start" gap="3" justify="center">
                  Add new integration
                </Flex>
              </Heading>
              <Flex wrap="wrap" align="start" gap="3" width="100%">
                {nonConfiguredIntegrations &&
                  Object.entries(nonConfiguredIntegrations).map(
                    ([_projectPath, integration], index) => {
                      return (
                        <IntegrationCard
                          isNotConfigured
                          key={`project-${index}-${JSON.stringify(
                            integration.integr_config_path,
                          )}`}
                          integration={integration}
                          handleIntegrationShowUp={handleIntegrationShowUp}
                        />
                      );
                    },
                  )}
              </Flex>
            </Flex>
          </Flex>
        )}
      </Flex>
    </Box>
  );
};
