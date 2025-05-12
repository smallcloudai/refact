import { FetchBaseQueryError } from "@reduxjs/toolkit/query";
import isEqual from "lodash.isequal";
import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { debugIntegrations } from "../../../debugConfig";
import { setIntegrationData } from "../../../features/Chat";
import { selectConfig } from "../../../features/Config/configSlice";
import {
  clearError,
  getErrorMessage,
  setError,
} from "../../../features/Errors/errorsSlice";
import {
  clearInformation,
  getInformationMessage,
  setInformation,
} from "../../../features/Errors/informationSlice";
import { convertRawIntegrationFormValues } from "../../../features/Integrations/convertRawIntegrationFormValues";
import {
  IntegrationsSetupPage,
  isIntegrationSetupPage,
  pop,
  popBackTo,
  selectCurrentPage,
} from "../../../features/Pages/pagesSlice";
import { useAppDispatch, useAppSelector } from "../../../hooks";
import { useDeleteIntegrationByPath } from "../../../hooks/useDeleteIntegrationByPath";
import { useSaveIntegrationData } from "../../../hooks/useSaveIntegrationData";
import {
  areAllFieldsBoolean,
  areIntegrationsNotConfigured,
  areToolConfirmation,
  areToolParameters,
  dockerApi,
  GroupedIntegrationWithIconRecord,
  Integration,
  integrationsApi,
  IntegrationWithIconRecord,
  IntegrationWithIconRecordAndAddress,
  IntegrationWithIconResponse,
  isDetailMessage,
  isDictionary,
  isMCPArgumentsArray,
  isMCPEnvironmentsDict,
  isNotConfiguredIntegrationWithIconRecord,
  isPrimitive,
  MCPArgs,
  MCPEnvs,
  NotConfiguredIntegrationWithIconRecord,
  ToolConfirmation,
  ToolParameterEntity,
} from "../../../services/refact";
import { toPascalCase } from "../../../utils/toPascalCase";
import { validateSnakeCase } from "../../../utils/validateSnakeCase";
import { formatIntegrationIconPath } from "../../../utils/formatIntegrationIconPath";

type useIntegrationsViewArgs = {
  integrationsMap?: IntegrationWithIconResponse;
  handleIfInnerIntegrationWasSet: (state: boolean) => void;
  goBack?: () => void;
};

export const INTEGRATIONS_WITH_TERMINAL_ICON = ["cmdline", "service", "mcp"];

export const useIntegrations = ({
  integrationsMap,
  handleIfInnerIntegrationWasSet,
  goBack,
}: useIntegrationsViewArgs) => {
  const dispatch = useAppDispatch();
  const globalError = useAppSelector(getErrorMessage);
  const information = useAppSelector(getInformationMessage);

  const { saveIntegrationMutationTrigger } = useSaveIntegrationData();
  // const currentThreadIntegration = useAppSelector(selectIntegration);

  const { deleteIntegrationTrigger } = useDeleteIntegrationByPath();

  const currentPage = useAppSelector(selectCurrentPage);
  const currentThreadIntegration = useMemo(() => {
    if (!currentPage) return null;
    if (!isIntegrationSetupPage(currentPage)) return null;
    return currentPage;
  }, [currentPage]);
  const isTemplateIntegration = (
    integrationName: string | undefined,
    type: "cmdline" | "service",
  ): boolean => {
    return integrationName?.startsWith(type) ?? false;
  };

  const getCommandName = (
    integrationName: string | undefined,
    isCmdline: boolean,
    isService: boolean,
  ): string | undefined => {
    if (!integrationName || (!isCmdline && !isService)) return undefined;
    return integrationName.split("_").slice(1).join("_");
  };

  const findIntegration = useCallback(
    (
      integrationsMap: IntegrationWithIconResponse,
      threadIntegration: IntegrationsSetupPage,
    ): IntegrationWithIconRecord | null => {
      const { integrationName, integrationPath, shouldIntermediatePageShowUp } =
        threadIntegration;
      const isCmdline = isTemplateIntegration(integrationName, "cmdline");
      const isService = isTemplateIntegration(integrationName, "service");

      // Handle template cases first
      if (!integrationPath && (isCmdline || isService)) {
        const templateName = `${isCmdline ? "cmdline" : "service"}_TEMPLATE`;
        return (
          integrationsMap.integrations.find(
            (i) => i.integr_name === templateName,
          ) ?? null
        );
      }

      // Handle regular integration search
      return (
        integrationsMap.integrations.find((integration) => {
          if (!shouldIntermediatePageShowUp) {
            return integrationName
              ? integration.integr_name === integrationName &&
                  integration.integr_config_path === integrationPath
              : integration.integr_config_path === integrationPath;
          }

          return integrationName
            ? integration.integr_name === integrationName
            : integration.integr_config_path === integrationPath;
        }) ?? null
      );
    },
    [],
  );

  const maybeIntegration = useMemo(() => {
    if (!currentThreadIntegration || !integrationsMap) return null;

    debugIntegrations(
      `[DEBUG LINKS]: currentThreadIntegration: `,
      currentThreadIntegration,
    );

    const integration = findIntegration(
      integrationsMap,
      currentThreadIntegration,
    );

    if (!integration) {
      debugIntegrations(`[DEBUG INTEGRATIONS] not found integration`);
      return null;
    }

    const isCmdline = isTemplateIntegration(
      currentThreadIntegration.integrationName,
      "cmdline",
    );
    const isService = isTemplateIntegration(
      currentThreadIntegration.integrationName,
      "service",
    );

    const integrationWithFlag: IntegrationWithIconRecordAndAddress = {
      ...integration,
      commandName: getCommandName(
        currentThreadIntegration.integrationName,
        isCmdline,
        isService,
      ),
      shouldIntermediatePageShowUp:
        currentThreadIntegration.shouldIntermediatePageShowUp ?? false,
      wasOpenedThroughChat:
        currentThreadIntegration.wasOpenedThroughChat ?? false,
    };

    debugIntegrations(
      `[DEBUG NAVIGATE]: integrationWithFlag: `,
      integrationWithFlag,
    );

    return integrationWithFlag;
  }, [currentThreadIntegration, integrationsMap, findIntegration]);

  // TBD: what if they went home then came back to integrations?

  const [currentIntegration, setCurrentIntegration] =
    useState<IntegrationWithIconRecord | null>(
      maybeIntegration?.shouldIntermediatePageShowUp ? null : maybeIntegration,
    );

  const [currentNotConfiguredIntegration, setCurrentNotConfiguredIntegration] =
    useState<NotConfiguredIntegrationWithIconRecord | null>(null);

  const config = useAppSelector(selectConfig);
  const port = config.lspPort;

  const integrationLogo = useMemo(() => {
    if (!currentIntegration && !currentNotConfiguredIntegration) {
      return "https://placehold.jp/150x150.png";
    }

    const iconPath = currentIntegration
      ? formatIntegrationIconPath(currentIntegration.icon_path)
      : currentNotConfiguredIntegration
        ? formatIntegrationIconPath(currentNotConfiguredIntegration.icon_path)
        : "";

    return iconPath
      ? `http://127.0.0.1:${port}/v1${iconPath}`
      : "https://placehold.jp/150x150.png";
  }, [currentIntegration, currentNotConfiguredIntegration, port]);

  // TODO: uncomment when ready
  useEffect(() => {
    if (!maybeIntegration) return;

    if (maybeIntegration.shouldIntermediatePageShowUp) {
      setCurrentNotConfiguredIntegration(() => {
        const similarIntegrations = integrationsMap?.integrations.filter(
          (integr) => integr.integr_name === maybeIntegration.integr_name,
        );
        if (!similarIntegrations) return null;

        const uniqueConfigPaths = Array.from(
          new Set(
            similarIntegrations.map((integr) => integr.integr_config_path),
          ),
        );
        const uniqueProjectPaths = Array.from(
          new Set(similarIntegrations.map((integr) => integr.project_path)),
        );

        uniqueProjectPaths.sort((a, _b) => (a === "" ? -1 : 1));
        uniqueConfigPaths.sort((a, _b) => (a.includes(".config") ? -1 : 1));

        const integrationToConfigure: NotConfiguredIntegrationWithIconRecord = {
          ...maybeIntegration,
          commandName: maybeIntegration.commandName
            ? maybeIntegration.commandName
            : undefined,
          wasOpenedThroughChat: maybeIntegration.shouldIntermediatePageShowUp,
          integr_config_path: uniqueConfigPaths,
          project_path: uniqueProjectPaths,
          integr_config_exists: false,
        };

        return integrationToConfigure;
      });
      setCurrentIntegration(null);
    } else {
      setCurrentIntegration(maybeIntegration);
      setCurrentNotConfiguredIntegration(null);
    }
  }, [maybeIntegration, integrationsMap?.integrations]);

  const [currentIntegrationSchema, setCurrentIntegrationSchema] = useState<
    Integration["integr_schema"] | null
  >(null);

  const [currentIntegrationValues, setCurrentIntegrationValues] = useState<
    Integration["integr_values"] | null
  >(null);

  const [isApplyingIntegrationForm, setIsApplyingIntegrationForm] =
    useState<boolean>(false);

  const [isDeletingIntegration, setIsDeletingIntegration] =
    useState<boolean>(false);

  const [isDisabledIntegrationForm, setIsDisabledIntegrationForm] =
    useState<boolean>(true);

  const [availabilityValues, setAvailabilityValues] = useState<
    Record<string, boolean>
  >({});

  const [confirmationRules, setConfirmationRules] = useState<ToolConfirmation>({
    ask_user: [],
    deny: [],
  });

  const [MCPArguments, setMCPArguments] = useState<MCPArgs>([]);
  const [MCPEnvironmentVariables, setMCPEnvironmentVariables] =
    useState<MCPEnvs>({});

  const [headers, setHeaders] = useState<Record<string, string>>({});

  const [toolParameters, setToolParameters] = useState<
    ToolParameterEntity[] | null
  >(null);

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

  const availableIntegrationsToConfigure = useMemo(() => {
    if (integrationsMap?.integrations) {
      const groupedIntegrations = integrationsMap.integrations.reduce<
        Record<string, GroupedIntegrationWithIconRecord>
      >((acc, integration) => {
        if (!(integration.integr_name in acc)) {
          acc[integration.integr_name] = {
            ...integration,
            project_path: [integration.project_path],
            integr_config_path: [integration.integr_config_path],
          };
        } else {
          acc[integration.integr_name].project_path.push(
            integration.project_path,
          );
          acc[integration.integr_name].integr_config_path.push(
            integration.integr_config_path,
          );
        }
        return acc;
      }, {});

      const filteredIntegrations = Object.values(groupedIntegrations).filter(
        areIntegrationsNotConfigured,
      );

      // Sort paths so that paths containing ".config" are first
      Object.values(filteredIntegrations).forEach((integration) => {
        integration.project_path.sort((a, _b) => (a === "" ? -1 : 1));
        integration.integr_config_path.sort((a, _b) =>
          a.includes(".config") ? -1 : 1,
        );
      });

      return Object.values(filteredIntegrations);
    }
  }, [integrationsMap]);

  useEffect(() => {
    debugIntegrations(
      `[DEBUG]: availableIntegrationsToConfigure: `,
      availableIntegrationsToConfigure,
    );
  }, [availableIntegrationsToConfigure]);

  // TODO: make this one in better way, too much of code
  useEffect(() => {
    if (!currentIntegrationValues) {
      setIsDisabledIntegrationForm(false);
    }
    if (
      currentIntegration &&
      currentIntegrationSchema &&
      currentIntegrationValues
    ) {
      setIsDisabledIntegrationForm((isDisabled) => {
        const isMCPIntegration = currentIntegration.integr_name.includes("mcp");
        const toolParametersChanged =
          toolParameters &&
          areToolParameters(currentIntegrationValues.parameters) &&
          !isMCPIntegration // if integration is MCP, then not checking toolParameters
            ? !isEqual(toolParameters, currentIntegrationValues.parameters)
            : false;

        const MCPArgumentsChanged = isMCPArgumentsArray(
          currentIntegrationValues.args,
        )
          ? !isEqual(currentIntegrationValues.args, MCPArguments)
          : false;

        const MCPEnvironmentVariablesChanged = isMCPEnvironmentsDict(
          currentIntegrationValues.env,
        )
          ? !isEqual(currentIntegrationValues.env, MCPEnvironmentVariables)
          : false;

        const headersChanged = isDictionary(currentIntegrationValues.headers)
          ? !isEqual(currentIntegrationValues.headers, headers)
          : false;

        const confirmationRulesChanged = !isEqual(
          confirmationRules,
          currentIntegrationValues.confirmation,
        );

        const someFieldsHaveBeenChanged =
          confirmationRulesChanged ||
          toolParametersChanged ||
          MCPArgumentsChanged ||
          MCPEnvironmentVariablesChanged ||
          headersChanged;

        // Manually collecting data from the form
        const formElement = document.getElementById(
          `form-${currentIntegration.integr_name}`,
        ) as HTMLFormElement | null;

        if (!formElement) return true;
        const formData = new FormData(formElement);
        const rawFormValues = Object.fromEntries(formData.entries());

        const formValues = convertRawIntegrationFormValues(
          rawFormValues,
          currentIntegrationSchema,
          currentIntegrationValues,
        );

        const otherFieldsChanged = !Object.entries(formValues).every(
          ([fieldKey, fieldValue]) => {
            if (isPrimitive(fieldValue)) {
              return (
                fieldKey in currentIntegrationValues &&
                fieldValue === currentIntegrationValues[fieldKey]
              );
            }
            if (typeof fieldValue === "object" || Array.isArray(fieldValue)) {
              return (
                fieldKey in currentIntegrationValues &&
                isEqual(fieldValue, currentIntegrationValues[fieldKey])
              );
            }
            return false;
          },
        );

        const allToolParametersAreWrittenInSnakeCase = toolParameters?.every(
          (param) => validateSnakeCase(param.name),
        );

        if (
          typeof allToolParametersAreWrittenInSnakeCase !== "undefined" &&
          !allToolParametersAreWrittenInSnakeCase
        ) {
          return true; // Disabling form if any of toolParameters are defined and not written in snake case
        }

        if (someFieldsHaveBeenChanged && isDisabled) {
          return false;
        }

        if (!otherFieldsChanged && !someFieldsHaveBeenChanged) {
          return true; // Disable form if all fields are back to original state
        }

        return isDisabled;
      });
    }
  }, [
    toolParameters,
    currentIntegrationValues,
    currentIntegrationSchema,
    confirmationRules,
    currentIntegration,
    MCPArguments,
    MCPEnvironmentVariables,
    headers,
  ]);

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
    setAvailabilityValues({});
    setToolParameters([]);
    setConfirmationRules({
      ask_user: [],
      deny: [],
    });
    setMCPArguments([]);
    setMCPEnvironmentVariables({});
    information && dispatch(clearInformation());
    globalError && dispatch(clearError());
    currentIntegrationValues && setCurrentIntegrationValues(null);
    currentIntegrationSchema && setCurrentIntegrationSchema(null);
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
    currentIntegrationValues,
    currentIntegrationSchema,
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

      const formValues = convertRawIntegrationFormValues(
        rawFormValues,
        currentIntegrationSchema,
        currentIntegrationValues,
      );

      debugIntegrations(`[DEBUG]: formValues: `, formValues);

      formValues.available = availabilityValues;
      if (
        currentIntegration.integr_name.includes("cmdline") ||
        currentIntegration.integr_name.includes("service")
      ) {
        formValues.parameters = toolParameters;
      }
      if (currentIntegration.integr_name.includes("mcp")) {
        formValues.env = MCPEnvironmentVariables;
        formValues.args = MCPArguments;
        formValues.headers = headers;
      }
      if (!currentIntegrationSchema.confirmation.not_applicable) {
        formValues.confirmation = confirmationRules;
      }

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
      currentIntegrationValues,
      dispatch,
      availabilityValues,
      confirmationRules,
      toolParameters,
      MCPArguments,
      MCPEnvironmentVariables,
      headers,
    ],
  );

  const handleDeleteIntegration = useCallback(
    async (configurationPath: string) => {
      if (!currentIntegration) return;
      setIsDeletingIntegration(true);
      const response = await deleteIntegrationTrigger(configurationPath);
      debugIntegrations("[DEBUG]: response: ", response);
      if (response.error) {
        debugIntegrations(`[DEBUG]: delete error: `, response.error);
        return;
      }
      dispatch(
        setInformation(
          `${toPascalCase(
            currentIntegration.integr_name,
          )} integration's configuration was deleted successfully!`,
        ),
      );
      const timeoutId = setTimeout(() => {
        setIsDeletingIntegration(false);
        handleFormReturn();
        clearTimeout(timeoutId);
      }, 1200);
    },
    [currentIntegration, dispatch, deleteIntegrationTrigger, handleFormReturn],
  );

  const handleIntegrationFormChange = useCallback(
    (event: FormEvent<HTMLFormElement>) => {
      if (!currentIntegration) return;
      if (!currentIntegrationSchema) return;
      // if (!currentIntegrationValues) return;
      // if (!currentIntegrationValues.available) return;
      event.preventDefault();

      const formData = new FormData(event.currentTarget);
      const rawFormValues = Object.fromEntries(formData.entries());

      // Adjust types of data based on f_type of each field in schema
      const formValues = convertRawIntegrationFormValues(
        rawFormValues,
        currentIntegrationSchema,
        currentIntegrationValues,
      );

      // formValues.parameters = toolParameters;

      const eachFormValueIsNotChanged = currentIntegrationValues
        ? Object.entries(formValues).every(([fieldKey, fieldValue]) => {
            if (isPrimitive(fieldValue)) {
              return (
                fieldKey in currentIntegrationValues &&
                fieldValue === currentIntegrationValues[fieldKey]
              );
            }
            if (typeof fieldValue === "object" || Array.isArray(fieldValue)) {
              return (
                fieldKey in currentIntegrationValues &&
                isEqual(fieldValue, currentIntegrationValues[fieldKey])
              );
            }
          })
        : false;

      debugIntegrations(
        `[DEBUG MCP]: eachFormValueIsNotChanged: `,
        eachFormValueIsNotChanged,
      );

      const eachAvailabilityOptionIsNotChanged = currentIntegrationValues
        ? Object.entries(availabilityValues).every(([fieldKey, fieldValue]) => {
            const availableObj = currentIntegrationValues.available;
            if (availableObj && areAllFieldsBoolean(availableObj)) {
              return (
                fieldKey in availableObj &&
                fieldValue === availableObj[fieldKey]
              );
            }
            return false;
          })
        : true;

      const eachToolParameterIsNotChanged =
        toolParameters &&
        currentIntegrationValues &&
        areToolParameters(currentIntegrationValues.parameters)
          ? isEqual(currentIntegrationValues.parameters, toolParameters)
          : true;

      const eachMCPArgumentIsNotChanged =
        currentIntegrationValues &&
        isMCPArgumentsArray(currentIntegrationValues.args)
          ? isEqual(currentIntegrationValues.args, MCPArguments)
          : true;

      const eachMCPEnvironmentVariableIsNotChanged =
        currentIntegrationValues &&
        isMCPEnvironmentsDict(currentIntegrationValues.env)
          ? isEqual(currentIntegrationValues.env, MCPEnvironmentVariables)
          : true;

      const eachToolConfirmationIsNotChanged =
        currentIntegrationValues &&
        areToolConfirmation(currentIntegrationValues.confirmation)
          ? isEqual(currentIntegrationValues.confirmation, confirmationRules)
          : true;
      debugIntegrations(`[DEBUG MCP]: formValues: `, formValues);
      debugIntegrations(
        `[DEBUG MCP]: currentIntegrationValues: `,
        currentIntegrationValues,
      );
      debugIntegrations(
        `[DEBUG MCP]: eachAvailabilityOptionIsNotChanged: `,
        eachAvailabilityOptionIsNotChanged,
      );

      debugIntegrations(
        `[DEBUG MCP]: eachToolParameterIsNotChanged: `,
        eachToolParameterIsNotChanged,
      );

      debugIntegrations(
        `[DEBUG MCP]: eachToolConfirmationIsNotChanged: `,
        eachToolConfirmationIsNotChanged,
      );
      debugIntegrations(
        `[DEBUG MCP]: availabilityValues: `,
        availabilityValues,
      );
      const maybeDisabled =
        eachFormValueIsNotChanged &&
        eachAvailabilityOptionIsNotChanged &&
        eachToolParameterIsNotChanged &&
        eachToolConfirmationIsNotChanged &&
        eachMCPArgumentIsNotChanged &&
        eachMCPEnvironmentVariableIsNotChanged;

      debugIntegrations(`[DEBUG MCP]: maybeDisabled: `, maybeDisabled);

      const areToolParametersWrittenInSnakeCase = toolParameters?.every(
        (param) => validateSnakeCase(param.name),
      );

      debugIntegrations(
        `[DEBUG MCP]: areToolParametersWrittenInSnakeCase: `,
        areToolParametersWrittenInSnakeCase,
      );

      const newDisabled =
        areToolParametersWrittenInSnakeCase !== undefined
          ? areToolParametersWrittenInSnakeCase
            ? maybeDisabled
            : true
          : maybeDisabled;

      debugIntegrations(`[DEBUG MCP]: newDisabled: `, newDisabled);

      setIsDisabledIntegrationForm(newDisabled);
    },
    [
      currentIntegration,
      currentIntegrationValues,
      currentIntegrationSchema,
      availabilityValues,
      toolParameters,
      confirmationRules,
      MCPArguments,
      MCPEnvironmentVariables,
    ],
  );

  useEffect(() => {
    debugIntegrations(`[DEBUG PARAMETERS]: toolParameters: `, toolParameters);
  }, [toolParameters]);

  const handleNotConfiguredIntegrationSubmit = useCallback(
    (event: FormEvent<HTMLFormElement>) => {
      if (!integrationsMap) return;
      if (!currentNotConfiguredIntegration) return;
      event.preventDefault();
      debugIntegrations(`[DEBUG]: event: `, event);
      const formData = new FormData(event.currentTarget);
      const rawFormValues = Object.fromEntries(formData.entries());
      debugIntegrations(`[DEBUG]: rawFormValues: `, rawFormValues);
      const [type, rest] =
        currentNotConfiguredIntegration.integr_name.split("_");
      if (
        "integr_config_path" in rawFormValues &&
        typeof rawFormValues.integr_config_path === "string" &&
        "command_name" in rawFormValues &&
        typeof rawFormValues.command_name === "string"
      ) {
        // making integration-get call and setting the result as currentIntegration
        const commandName = rawFormValues.command_name;
        const configPath = rawFormValues.integr_config_path.replace(
          rest,
          commandName,
        );

        debugIntegrations(
          `[DEBUG INTERMEDIATE PAGE]: config path for \`v1/integration-get\`: `,
          configPath,
        );

        const customIntegration: IntegrationWithIconRecord = {
          when_isolated: false,
          on_your_laptop: false,
          integr_name: `${type}_${commandName}`,
          integr_config_path: configPath,
          icon_path: currentNotConfiguredIntegration.icon_path,
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

  const goBackAndClearError = useCallback(() => {
    goBack && goBack();
    dispatch(clearError());
    setCurrentIntegration(null);
    setCurrentNotConfiguredIntegration(null);
    dispatch(setIntegrationData(null));
  }, [dispatch, goBack]);

  const handleNotSetupIntegrationShowUp = useCallback(
    (integration: NotConfiguredIntegrationWithIconRecord) => {
      if (!integrationsMap) return;

      debugIntegrations(
        `[DEBUG]: open form for not configured integration: `,
        integration,
      );

      setCurrentNotConfiguredIntegration(integration);
    },
    [integrationsMap],
  );

  const handleIntegrationShowUp = useCallback(
    (
      integration:
        | IntegrationWithIconRecord
        | NotConfiguredIntegrationWithIconRecord,
    ) => {
      if (isNotConfiguredIntegrationWithIconRecord(integration)) {
        handleNotSetupIntegrationShowUp(integration);
        return;
      }
      setCurrentIntegration(integration);
    },
    [handleNotSetupIntegrationShowUp],
  );

  useEffect(() => {
    debugIntegrations(`[DEBUG MCP]: MCPArguments: `, MCPArguments);
  }, [MCPArguments]);

  useEffect(() => {
    debugIntegrations(
      `[DEBUG MCP]: MCPEnvironmentVariables: `,
      MCPEnvironmentVariables,
    );
  }, [MCPEnvironmentVariables]);

  return {
    currentIntegration,
    currentIntegrationSchema,
    currentIntegrationValues,
    currentNotConfiguredIntegration,
    confirmationRules,
    toolParameters,
    availabilityValues,
    MCPArguments,
    MCPEnvironmentVariables,
    integrationLogo,
    handleFormReturn,
    handleIntegrationFormChange,
    handleSubmit,
    handleDeleteIntegration,
    handleNotConfiguredIntegrationSubmit,
    handleNavigateToIntegrationSetup,
    handleSetCurrentIntegrationSchema,
    handleSetCurrentIntegrationValues,
    goBackAndClearError,
    handleIntegrationShowUp,
    setAvailabilityValues,
    setConfirmationRules,
    setToolParameters,
    setMCPArguments,
    setMCPEnvironmentVariables,
    setHeaders,
    isDisabledIntegrationForm,
    isApplyingIntegrationForm,
    isDeletingIntegration,
    globalIntegrations,
    projectSpecificIntegrations,
    groupedProjectIntegrations,
    availableIntegrationsToConfigure,
  };
};
