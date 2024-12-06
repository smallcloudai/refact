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
    if (currentIntegration) {
      handleIfInnerIntegrationWasSet(true);
    } else {
      handleIfInnerIntegrationWasSet(false);
    }
  }, [currentIntegration, handleIfInnerIntegrationWasSet]);

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
      return integrationsMap.integrations.reduce<
        Record<string, IntegrationWithIconResponse["integrations"]>
      >((acc, integration) => {
        if (
          !integration.integr_config_exists &&
          !integration.integr_name.startsWith("cmdline") &&
          !integration.integr_name.startsWith("service")
        ) {
          if (!(integration.project_path in acc)) {
            acc[integration.project_path] = [];
          }
          acc[integration.project_path].push(integration);
        }
        return acc;
      }, {});
    }
  }, [integrationsMap]);

  const nonConfiguredCmdlinesIntegrations = useMemo(() => {
    if (integrationsMap?.integrations) {
      const groupedIntegrations = integrationsMap.integrations.reduce<
        Record<
          string,
          Omit<
            IntegrationWithIconRecord,
            "project_path" | "integr_config_path"
          > & {
            project_path: string[];
            integr_config_path: string[];
          }
        >
      >((acc, integration) => {
        if (
          !integration.integr_config_exists &&
          (integration.integr_name.startsWith("cmdline") ||
            integration.integr_name.startsWith("service"))
        ) {
          if (!(integration.integr_name in acc)) {
            acc[integration.integr_name] = {
              ...integration,
              project_path: [integration.project_path],
              integr_config_path: [integration.integr_config_path],
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

      return Object.values(groupedIntegrations);
    }
  }, [integrationsMap]);

  useEffect(() => {
    debugIntegrations(
      `[DEBUG]: nonConfiguredCmdlinesIntegrations: `,
      nonConfiguredCmdlinesIntegrations,
    );
  }, [nonConfiguredCmdlinesIntegrations]);

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
    information && dispatch(clearInformation());
    globalError && dispatch(clearError());
    dispatch(integrationsApi.util.resetApiState());
    dispatch(dockerApi.util.resetApiState());
    // TODO: can cause a loop where integration pages goes back to form
    dispatch(pop());
    dispatch(popBackTo({ name: "integrations page" }));
  }, [dispatch, globalError, information, currentIntegration]);

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
          default:
            acc[key] = rawFormValues[key] as string;
            break;
        }
        return acc;
      }, {});

      const eachFormValueIsNotChanged = Object.entries(formValues).every(
        ([fieldKey, fieldValue]) => {
          return (
            fieldKey in currentIntegrationValues &&
            fieldValue === currentIntegrationValues[fieldKey]
          );
        },
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

  const integrationLogo = useMemo(() => {
    if (!currentIntegration) return "https://placehold.jp/150x150.png";
    return INTEGRATIONS_WITH_TERMINAL_ICON.includes(
      currentIntegration.integr_name.split("_")[0],
    )
      ? iconMap.cmdline
      : iconMap[currentIntegration.integr_name];
  }, [currentIntegration]);

  if (isLoading) {
    return <Spinner spinning />;
  }

  const goBackAndClearError = () => {
    goBack && goBack();
    dispatch(clearError());
    setCurrentIntegration(null);
  };

  const handleIntegrationShowUp = (
    integration: IntegrationWithIconResponse["integrations"][number],
  ) => {
    debugIntegrations(`[DEBUG]: open form: `, integration);
    setCurrentIntegration(integration);
  };
  const handleNotSetupIntegrationShowUp = (
    integration: IntegrationWithIconResponse["integrations"][number],
  ) => {
    if (!integrationsMap) return;

    debugIntegrations(
      `[DEBUG]: open form for not configured integration: `,
      integration,
    );

    const maybeConfiguredGlobalIntegration = integrationsMap.integrations.find(
      (integr) =>
        integr.integr_name === integration.integr_name &&
        integr.project_path === "" &&
        integr.integr_config_exists,
    );
    const maybeConfiguredLocalIntegration = integrationsMap.integrations.find(
      (integr) =>
        integr.integr_name === integration.integr_name &&
        integr.project_path === integration.project_path &&
        integr.integr_config_exists,
    );

    if (!maybeConfiguredGlobalIntegration && !maybeConfiguredLocalIntegration) {
      debugIntegrations(
        `[DEBUG]: no locally neither globally configured ${integration.integr_name} were found. asking to choose to configure local or global configuration`,
      );
      return;
    }
    if (maybeConfiguredGlobalIntegration) {
      debugIntegrations(
        `[DEBUG]: found globally configured ${maybeConfiguredGlobalIntegration.integr_name} integration! should configure local configuration`,
      );
    }

    if (maybeConfiguredLocalIntegration) {
      debugIntegrations(
        `[DEBUG]: found locally configured ${maybeConfiguredLocalIntegration.integr_name} integration! should configure global configuration`,
      );
    }

    setCurrentIntegration(integration);
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
        {currentIntegration && (
          <IntegrationsHeader
            leftRightPadding={leftRightPadding}
            handleFormReturn={handleFormReturn}
            integrationName={currentIntegration.integr_name}
            icon={integrationLogo}
          />
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
        {!currentIntegration && (
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

            {nonConfiguredIntegrations &&
              Object.entries(nonConfiguredIntegrations).map(
                ([_projectPath, integrations], index) => {
                  return (
                    <Flex
                      key={`project-group-${index}`}
                      direction="column"
                      gap="4"
                      align="start"
                    >
                      <Heading as="h4" size="3">
                        <Flex align="start" gap="3" justify="center">
                          Add new integration
                        </Flex>
                      </Heading>
                      <Flex wrap="wrap" align="start" gap="3" width="100%">
                        {integrations.map((integration, subIndex) => (
                          <IntegrationCard
                            isInline
                            key={`project-${index}-${subIndex}-${integration.integr_config_path}`}
                            integration={integration}
                            handleIntegrationShowUp={
                              handleNotSetupIntegrationShowUp
                            }
                          />
                        ))}
                        {nonConfiguredCmdlinesIntegrations?.map(
                          (integration, subIndex) => (
                            <IntegrationCard
                              isInline
                              key={`project-${index}-${subIndex}-${JSON.stringify(
                                integration.integr_config_path,
                              )}`}
                              integration={integration}
                              handleIntegrationShowUp={
                                handleNotSetupIntegrationShowUp
                              }
                            />
                          ),
                        )}
                      </Flex>
                    </Flex>
                  );
                },
              )}
          </Flex>
        )}
      </Flex>
    </Box>
  );
};
