import { Box, Flex, Heading, Text, Grid } from "@radix-ui/themes";
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
  isNotConfiguredIntegrationWithIconRecord,
  isPrimitive,
  NotConfiguredIntegrationWithIconRecord,
  ToolConfirmation,
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
import { useDeleteIntegrationByPath } from "../../hooks/useDeleteIntegrationByPath";
import { toPascalCase } from "../../utils/toPascalCase";
import { selectThemeMode } from "../../features/Config/configSlice";
import type { ToolParameterEntity } from "../../services/refact";
import isEqual from "lodash.isequal";
import { convertRawIntegrationFormValues } from "../../features/Integrations/convertRawIntegrationFormValues";
import { validateSnakeCase } from "../../utils/validateSnakeCase";
import { setIntegrationData } from "../../features/Chat";

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

  const { deleteIntegrationTrigger } = useDeleteIntegrationByPath();

  const maybeIntegration = useMemo(() => {
    if (!currentThreadIntegration) return null;
    if (!integrationsMap) return null;
    debugIntegrations(
      `[DEBUG LINKS]: currentThreadIntegration: `,
      currentThreadIntegration,
    );
    const integrationName = currentThreadIntegration.integrationName;
    const integrationPath = currentThreadIntegration.integrationPath;
    const isCmdline = integrationName
      ? integrationName.startsWith("cmdline")
      : false;
    const isService = integrationName
      ? integrationName.startsWith("service")
      : false;
    const shouldIntermediatePageShowUp =
      currentThreadIntegration.shouldIntermediatePageShowUp;

    // TODO: check for extra flag in currentThreadIntegration to return different find() call from notConfiguredGrouped integrations if it's set to true
    const integration =
      integrationsMap.integrations.find((integration) => {
        if (!integrationPath) {
          if (isCmdline) return integration.integr_name === "cmdline_TEMPLATE";
          if (isService) return integration.integr_name === "service_TEMPLATE";
        }
        if (!shouldIntermediatePageShowUp)
          return integrationName
            ? integration.integr_name === integrationName &&
                integration.integr_config_path === integrationPath
            : integration.integr_config_path === integrationPath;
        return integrationName
          ? integration.integr_name === integrationName
          : integration.integr_config_path === integrationPath;
      }) ?? null;
    if (!integration) {
      debugIntegrations(`[DEBUG INTEGRATIONS] not found integration`);
      return null;
    }

    const integrationWithFlag = {
      ...integration,
      commandName:
        (isCmdline || isService) && integrationName
          ? integrationName.split("_").slice(1).join("_")
          : undefined,
      shouldIntermediatePageShowUp: shouldIntermediatePageShowUp ?? false,
      wasOpenedThroughChat:
        currentThreadIntegration.wasOpenedThroughChat ?? false,
    } as IntegrationWithIconRecordAndAddress;
    debugIntegrations(
      `[DEBUG NAVIGATE]: integrationWithFlag: `,
      integrationWithFlag,
    );
    return integrationWithFlag;
  }, [currentThreadIntegration, integrationsMap]);

  // TBD: what if they went home then came back to integrations?

  const [currentIntegration, setCurrentIntegration] =
    useState<IntegrationWithIconRecord | null>(
      maybeIntegration?.shouldIntermediatePageShowUp ? null : maybeIntegration,
    );

  const [currentNotConfiguredIntegration, setCurrentNotConfiguredIntegration] =
    useState<NotConfiguredIntegrationWithIconRecord | null>(null);

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
    if (
      currentIntegration &&
      currentIntegrationSchema &&
      currentIntegrationValues
    ) {
      setIsDisabledIntegrationForm((isDisabled) => {
        const toolParametersChanged =
          toolParameters &&
          areToolParameters(currentIntegrationValues.parameters)
            ? !isEqual(toolParameters, currentIntegrationValues.parameters)
            : false;

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

        const confirmationRulesChanged = !isEqual(
          confirmationRules,
          currentIntegrationValues.confirmation,
        );

        debugIntegrations(
          `[DEBUG confirmationRulesChanged]: confirmationRulesChanged: `,
          confirmationRulesChanged,
        );

        const allToolParametersNamesInSnakeCase = toolParameters
          ? toolParameters.every((param) => validateSnakeCase(param.name))
          : true;

        if (!allToolParametersNamesInSnakeCase) {
          return true; // Disabling form if any of tool parameters names are written not in snake case
        }

        if ((toolParametersChanged || confirmationRulesChanged) && isDisabled) {
          return false; // Enable form if toolParameters changed and form was disabled
        }

        if (
          otherFieldsChanged &&
          (toolParametersChanged || confirmationRulesChanged)
        ) {
          return isDisabled; // Keep the form in the same condition
        }

        if (
          !otherFieldsChanged &&
          !toolParametersChanged &&
          !confirmationRulesChanged
        ) {
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
    ],
  );

  const handleDeleteIntegration = useCallback(
    async (configurationPath: string, integrationName: string) => {
      // if (!currentIntegration) return;
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
            integrationName,
          )} integration's configuration was deleted successfully!`,
        ),
      );
      const timeoutId = setTimeout(() => {
        setIsDeletingIntegration(false);
        handleFormReturn();
        clearTimeout(timeoutId);
      }, 1200);
    },
    [dispatch, deleteIntegrationTrigger, handleFormReturn],
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
        `[DEBUG]: eachFormValueIsNotChanged: `,
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

      const eachToolConfirmationIsNotChanged =
        currentIntegrationValues &&
        areToolConfirmation(currentIntegrationValues.confirmation)
          ? isEqual(currentIntegrationValues.confirmation, confirmationRules)
          : true;
      debugIntegrations(`[DEBUG]: formValues: `, formValues);
      debugIntegrations(
        `[DEBUG]: currentIntegrationValues: `,
        currentIntegrationValues,
      );
      debugIntegrations(
        `[DEBUG]: eachAvailabilityOptionIsNotChanged: `,
        eachAvailabilityOptionIsNotChanged,
      );

      debugIntegrations(
        `[DEBUG]: eachToolParameterIsNotChanged: `,
        eachToolParameterIsNotChanged,
      );

      debugIntegrations(
        `[DEBUG]: eachToolConfirmationIsNotChanged: `,
        eachToolConfirmationIsNotChanged,
      );
      debugIntegrations(`[DEBUG]: availabilityValues: `, availabilityValues);
      const maybeDisabled =
        eachFormValueIsNotChanged &&
        eachAvailabilityOptionIsNotChanged &&
        eachToolParameterIsNotChanged &&
        eachToolConfirmationIsNotChanged;

      debugIntegrations(`[DEBUG CHANGE]: maybeDisabled: `, maybeDisabled);

      setIsDisabledIntegrationForm(
        toolParameters
          ? toolParameters.every((param) => validateSnakeCase(param.name))
            ? maybeDisabled
            : true
          : maybeDisabled,
      );
    },
    [
      currentIntegration,
      currentIntegrationValues,
      currentIntegrationSchema,
      availabilityValues,
      toolParameters,
      confirmationRules,
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

  const theme = useAppSelector(selectThemeMode);
  const icons = iconMap(
    theme ? (theme === "inherit" ? "light" : theme) : "light",
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
      ? icons.cmdline
      : icons[
          currentIntegration
            ? currentIntegration.integr_name
            : currentNotConfiguredIntegration
              ? currentNotConfiguredIntegration.integr_name
              : ""
        ];
  }, [currentIntegration, currentNotConfiguredIntegration, icons]);

  if (isLoading) {
    return <Spinner spinning />;
  }

  const goBackAndClearError = () => {
    goBack && goBack();
    dispatch(clearError());
    setCurrentIntegration(null);
    setCurrentNotConfiguredIntegration(null);
    dispatch(setIntegrationData(null));
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
            handleInstantReturn={goBackAndClearError}
            instantBackReturnment={
              currentNotConfiguredIntegration
                ? currentNotConfiguredIntegration.wasOpenedThroughChat
                : currentIntegration
                  ? currentIntegration.wasOpenedThroughChat
                  : false
            }
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
              handleDeleteIntegration={(path: string, name: string) =>
                void handleDeleteIntegration(path, name)
              }
              integrationPath={currentIntegration.integr_config_path}
              isApplying={isApplyingIntegrationForm}
              isDeletingIntegration={isDeletingIntegration}
              isDisabled={isDisabledIntegrationForm}
              onSchema={handleSetCurrentIntegrationSchema}
              onValues={handleSetCurrentIntegrationValues}
              handleChange={handleIntegrationFormChange}
              availabilityValues={availabilityValues}
              confirmationRules={confirmationRules}
              setAvailabilityValues={setAvailabilityValues}
              setConfirmationRules={setConfirmationRules}
              setToolParameters={setToolParameters}
              handleSwitchIntegration={handleNavigateToIntegrationSetup}
            />
            {information && (
              <InformationCallout
                timeout={isDeletingIntegration ? 1000 : 3000}
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
                        <Flex
                          align="start"
                          gapX="3"
                          gapY="1"
                          justify="start"
                          wrap="wrap"
                        >
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
              <Grid
                align="stretch"
                gap="3"
                columns={{ initial: "2", xs: "3", sm: "4", md: "5" }}
                width="100%"
              >
                {availableIntegrationsToConfigure &&
                  Object.entries(availableIntegrationsToConfigure).map(
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
              </Grid>
            </Flex>
          </Flex>
        )}
      </Flex>
    </Box>
  );
};
