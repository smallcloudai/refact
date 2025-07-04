import { FetchBaseQueryError } from "@reduxjs/toolkit/query";
import isEqual from "lodash.isequal";
import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { debugIntegrations } from "../../../debugConfig";
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
  areIntegrationsNotConfigured,
  dockerApi,
  GroupedIntegrationWithIconRecord,
  Integration,
  IntegrationFieldValue,
  integrationsApi,
  IntegrationWithIconRecord,
  IntegrationWithIconRecordAndAddress,
  IntegrationWithIconResponse,
  isDetailMessage,
  isNotConfiguredIntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
  ToolParameterEntity,
} from "../../../services/refact";
import { toPascalCase } from "../../../utils/toPascalCase";
import { validateSnakeCase } from "../../../utils/validateSnakeCase";
import { formatIntegrationIconPath } from "../../../utils/formatIntegrationIconPath";
import { prepareNotConfiguredIntegration } from "../utils/prepareNotConfiguredIntegration";
// import groupBy from "lodash.groupby";

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
    const iconPath = currentIntegration
      ? formatIntegrationIconPath(currentIntegration.icon_path)
      : currentNotConfiguredIntegration
        ? formatIntegrationIconPath(currentNotConfiguredIntegration.icon_path)
        : "";

    return `http://127.0.0.1:${port}/v1${iconPath}`;
  }, [currentIntegration, currentNotConfiguredIntegration, port]);

  // This useEffect is required to decide whether or not the integration should be opened in intermediate page
  useEffect(() => {
    if (!maybeIntegration) return;

    if (maybeIntegration.shouldIntermediatePageShowUp) {
      setCurrentNotConfiguredIntegration(() => {
        return prepareNotConfiguredIntegration(
          maybeIntegration,
          integrationsMap?.integrations,
        );
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
  const [formValues, setFormValues] =
    useState<Integration["integr_values"]>(null);

  const [isApplyingIntegrationForm, setIsApplyingIntegrationForm] =
    useState<boolean>(false);

  const [isDeletingIntegration, setIsDeletingIntegration] =
    useState<boolean>(false);

  const [isDisabledIntegrationForm, setIsDisabledIntegrationForm] =
    useState<boolean>(true);

  useEffect(() => {
    debugIntegrations(`[DEBUG]: integrationsData: `, integrationsMap);
  }, [integrationsMap]);

  // Required for paddings in PageWrapper
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

  // Combined integration processing for better readability and maintainability
  const integrationsData = useMemo(() => {
    if (!integrationsMap?.integrations) {
      return {
        global: undefined,
        specific: undefined,
        configurable: undefined,
      };
    }

    const { integrations } = integrationsMap;

    // Helper function to filter global integrations
    const getGlobalIntegrations = () => {
      return integrations.filter(
        (integration) =>
          integration.project_path === "" && integration.integr_config_exists,
      );
    };

    // Helper function to filter and group project-specific integrations
    const getGroupedProjectIntegrations = () => {
      const projectSpecific = integrations.filter(
        (integration) => integration.project_path !== "",
      );
      return projectSpecific.reduce<
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
    };

    // Helper function to get available integrations to configure
    const getAvailableIntegrationsToConfigure = () => {
      // Group integrations by name to handle multiple paths/configs
      const groupedIntegrations = integrations.reduce<
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

      // Filter to only non-configured integrations
      const filteredIntegrations = Object.values(groupedIntegrations).filter(
        areIntegrationsNotConfigured,
      );

      // Sort paths so that paths containing ".config" are first
      filteredIntegrations.forEach((integration) => {
        integration.project_path.sort((a, _b) => (a === "" ? -1 : 1));
        integration.integr_config_path.sort((a, _b) =>
          a.includes(".config") ? -1 : 1,
        );
      });

      return filteredIntegrations;
    };

    return {
      global: getGlobalIntegrations(),
      specific: getGroupedProjectIntegrations(),
      configurable: getAvailableIntegrationsToConfigure(),
    };
  }, [integrationsMap]);

  useEffect(() => {
    if (!currentIntegrationValues) {
      setIsDisabledIntegrationForm(false);
    }
    if (
      currentIntegration &&
      currentIntegrationSchema &&
      currentIntegrationValues &&
      formValues
    ) {
      setIsDisabledIntegrationForm(() => {
        const someFieldsHaveBeenChanged = !isEqual(
          currentIntegrationValues,
          formValues,
        );

        const toolParameters = formValues.parameters as
          | ToolParameterEntity[]
          | undefined;
        const allToolParametersAreWrittenInSnakeCase = toolParameters?.every(
          (param) => validateSnakeCase(param.name),
        );

        if (
          typeof allToolParametersAreWrittenInSnakeCase !== "undefined" &&
          !allToolParametersAreWrittenInSnakeCase
        ) {
          return true; // Disabling form if any of toolParameters are defined and not written in snake case
        }

        return !someFieldsHaveBeenChanged;
      });
    }
  }, [
    currentIntegrationValues,
    currentIntegrationSchema,
    currentIntegration,
    formValues,
  ]);

  const handleSetCurrentIntegrationSchema = useCallback(
    (schema: Integration["integr_schema"]) => {
      if (!currentIntegration) return;

      setCurrentIntegrationSchema(schema);
    },
    [currentIntegration],
  );

  const handleSetCurrentIntegrationValues = useCallback(
    (values: Integration["integr_values"]) => {
      if (!currentIntegration) return;

      setCurrentIntegrationValues(values);
      setFormValues(values);
    },
    [currentIntegration],
  );

  const handleUpdateFormField = useCallback(
    (fieldKey: string, fieldValue: IntegrationFieldValue) => {
      setFormValues((prev) => {
        return { ...prev, [fieldKey]: fieldValue };
      });
    },
    [],
  );

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
    if (currentIntegrationValues) {
      setCurrentIntegrationValues(null);
      setFormValues(null);
    }
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
      formValues,
      dispatch,
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

  const handleNotConfiguredIntegrationSubmit = useCallback(
    (event: FormEvent<HTMLFormElement>) => {
      if (!integrationsMap) return;
      if (!currentNotConfiguredIntegration) return;
      event.preventDefault();
      const formData = new FormData(event.currentTarget);
      const rawFormValues = Object.fromEntries(formData.entries());

      const integrationType =
        currentNotConfiguredIntegration.integr_name.replace("_TEMPLATE", "");

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
          `[DEBUG INTERMEDIATE PAGE]: config path for \`v1/integration-get\`: `,
          configPath,
        );

        const newIntegration: IntegrationWithIconRecord = {
          when_isolated: false,
          on_your_laptop: false,
          integr_name: `${integrationType}_${commandName}`,
          integr_config_path: configPath,
          icon_path: currentNotConfiguredIntegration.icon_path,
          project_path: rawFormValues.integr_config_path
            .toString()
            .includes(".config")
            ? ""
            : rawFormValues.integr_config_path.toString(),
          integr_config_exists: false,
        };

        setCurrentIntegration(newIntegration);
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
          `[DEBUG]: Unexpected error occurred. It's mostly a bug`,
        );
      }
    },
    [currentNotConfiguredIntegration, integrationsMap],
  );

  const handleNavigateToIntegrationSetup = useCallback(
    (integrationName: string, _integrationConfigPath: string) => {
      if (!integrationsMap) return;
      if (!currentIntegration) return;

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

  return {
    currentIntegration,
    currentIntegrationSchema,
    currentIntegrationValues,
    currentNotConfiguredIntegration,
    integrationLogo,
    handleFormReturn,
    handleSubmit,
    handleDeleteIntegration,
    handleNotConfiguredIntegrationSubmit,
    handleNavigateToIntegrationSetup,
    handleSetCurrentIntegrationSchema,
    handleSetCurrentIntegrationValues,
    goBackAndClearError,
    handleIntegrationShowUp,
    handleUpdateFormField,
    isDisabledIntegrationForm,
    isApplyingIntegrationForm,
    isDeletingIntegration,
    globalIntegrations: integrationsData.global,
    groupedProjectIntegrations: integrationsData.specific,
    availableIntegrationsToConfigure: integrationsData.configurable,
    formValues,
  };
};
