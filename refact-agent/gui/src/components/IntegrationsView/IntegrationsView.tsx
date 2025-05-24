import { Box, Container, Flex } from "@radix-ui/themes";
import { FC, ReactNode } from "react";
import { clearError, getErrorMessage } from "../../features/Errors/errorsSlice";
import {
  clearInformation,
  getInformationMessage,
} from "../../features/Errors/informationSlice";
import { LeftRightPadding } from "../../features/Integrations/Integrations";
import { useAppDispatch, useAppSelector } from "../../hooks";
import { IntegrationWithIconResponse } from "../../services/refact";
import { ErrorCallout } from "../Callout";
import { InformationCallout } from "../Callout/Callout";
import { Spinner } from "../Spinner";
import { IntegrationsList } from "./DisplayIntegrations/IntegrationsList";
import { IntegrationForm } from "./IntegrationForm";
import { IntegrationsHeader } from "./Header/IntegrationsHeader";
import styles from "./IntegrationsView.module.css";
import { IntermediateIntegration } from "./IntermediateIntegration";
import { useIntegrations } from "./hooks/useIntegrations";
import classNames from "classnames";
import { selectConfig } from "../../features/Config/configSlice";

type IntegrationViewProps = {
  integrationsMap?: IntegrationWithIconResponse;
  leftRightPadding: LeftRightPadding;
  isLoading: boolean;
  goBack?: () => void;
  handleIfInnerIntegrationWasSet: (state: boolean) => void;
};

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

  const config = useAppSelector(selectConfig);

  const {
    currentIntegration,
    currentNotConfiguredIntegration,
    availableIntegrationsToConfigure,
    confirmationRules,
    availabilityValues,
    MCPArguments,
    MCPEnvironmentVariables,
    integrationLogo,
    handleIntegrationFormChange,
    handleSubmit,
    handleDeleteIntegration,
    handleNotConfiguredIntegrationSubmit,
    handleNavigateToIntegrationSetup,
    handleSetCurrentIntegrationSchema,
    handleSetCurrentIntegrationValues,
    handleFormReturn,
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
    groupedProjectIntegrations,
  } = useIntegrations({
    integrationsMap,
    handleIfInnerIntegrationWasSet,
    goBack,
  });

  const renderHeader = (): ReactNode => {
    if (!(currentIntegration ?? currentNotConfiguredIntegration)) return null;

    return (
      <IntegrationsHeader
        leftRightPadding={leftRightPadding}
        handleFormReturn={handleFormReturn}
        handleInstantReturn={goBackAndClearError}
        instantBackReturnment={
          currentNotConfiguredIntegration?.wasOpenedThroughChat ??
          currentIntegration?.wasOpenedThroughChat ??
          false
        }
        integrationName={
          currentIntegration?.integr_name ??
          currentNotConfiguredIntegration?.integr_name ??
          ""
        }
        icon={integrationLogo}
      />
    );
  };

  const renderIntegrationForm = ({
    currentHost,
  }: {
    currentHost: string;
  }): ReactNode => {
    if (!currentIntegration) return null;

    return (
      <Flex direction="column" align="start" justify="between" height="100%">
        <IntegrationForm
          handleSubmit={(event) => void handleSubmit(event)}
          handleDeleteIntegration={(path) => void handleDeleteIntegration(path)}
          integrationPath={currentIntegration.integr_config_path}
          isApplying={isApplyingIntegrationForm}
          isDeletingIntegration={isDeletingIntegration}
          isDisabled={isDisabledIntegrationForm}
          onSchema={handleSetCurrentIntegrationSchema}
          onValues={handleSetCurrentIntegrationValues}
          handleChange={handleIntegrationFormChange}
          availabilityValues={availabilityValues}
          confirmationRules={confirmationRules}
          MCPArguments={MCPArguments}
          MCPEnvironmentVariables={MCPEnvironmentVariables}
          setAvailabilityValues={setAvailabilityValues}
          setConfirmationRules={setConfirmationRules}
          setMCPArguments={setMCPArguments}
          setMCPEnvironmentVariables={setMCPEnvironmentVariables}
          setHeaders={setHeaders}
          setToolParameters={setToolParameters}
          handleSwitchIntegration={handleNavigateToIntegrationSetup}
        />
        {information && (
          <InformationCallout
            timeout={isDeletingIntegration ? 1000 : 3000}
            mx="0"
            onClick={() => dispatch(clearInformation())}
            className={classNames(styles.popup, {
              [styles.popup_ide]: currentHost !== "web",
            })}
          >
            {information}
          </InformationCallout>
        )}
        {globalError && (
          <ErrorCallout
            mx="0"
            timeout={3000}
            onClick={() => dispatch(clearError())}
            className={classNames(styles.popup, {
              [styles.popup_ide]: currentHost !== "web",
            })}
          >
            {globalError}
          </ErrorCallout>
        )}
      </Flex>
    );
  };

  const renderNotConfiguredIntegration = (): ReactNode => {
    if (!currentNotConfiguredIntegration) return null;

    return (
      <Flex direction="column" align="start" justify="between" height="100%">
        <IntermediateIntegration
          handleSubmit={handleNotConfiguredIntegrationSubmit}
          integration={currentNotConfiguredIntegration}
        />
      </Flex>
    );
  };

  if (isLoading) {
    return <Spinner spinning />;
  }

  if (!integrationsMap) {
    return (
      <ErrorCallout
        className={classNames(styles.popup, {
          [styles.popup_ide]: config.host !== "web",
        })}
        mx="0"
        onClick={goBackAndClearError}
      >
        fetching integrations.
      </ErrorCallout>
    );
  }

  const renderContent = (): ReactNode => {
    if (currentNotConfiguredIntegration) {
      return renderNotConfiguredIntegration();
    }

    if (currentIntegration) {
      return renderIntegrationForm({ currentHost: config.host });
    }

    return (
      <IntegrationsList
        globalIntegrations={globalIntegrations}
        availableIntegrationsToConfigure={availableIntegrationsToConfigure}
        groupedProjectIntegrations={groupedProjectIntegrations}
        handleIntegrationShowUp={handleIntegrationShowUp}
      />
    );
  };

  return (
    <Container px="1">
      <Box style={{ width: "inherit", height: "100%" }}>
        <Flex direction="column" style={{ width: "100%", height: "100%" }}>
          {renderHeader()}
          {renderContent()}
          {globalError && (
            <ErrorCallout
              mx="0"
              timeout={3000}
              onClick={() => dispatch(clearError())}
              className={classNames(styles.popup, {
                [styles.popup_ide]: config.host !== "web",
              })}
              preventRetry
            >
              {globalError}
            </ErrorCallout>
          )}
        </Flex>
      </Box>
    </Container>
  );
};
