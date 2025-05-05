import React, { useCallback, useState } from "react";
import { Flex } from "@radix-ui/themes";

import { ConfiguredProvidersView } from "./ConfiguredProvidersView";

import type {
  ConfiguredProvidersResponse,
  SimplifiedProvider,
} from "../../../services/refact";
import { ProviderPreview } from "../ProviderPreview";
import {
  ErrorCallout,
  InformationCallout,
} from "../../../components/Callout/Callout";
import classNames from "classnames";
import { useAppDispatch, useAppSelector } from "../../../hooks";
import { clearError, getErrorMessage } from "../../Errors/errorsSlice";
import {
  clearInformation,
  getInformationMessage,
} from "../../Errors/informationSlice";

import styles from "./ProvidersView.module.css";
import { selectConfig } from "../../Config/configSlice";

export type ProvidersViewProps = {
  configuredProviders: ConfiguredProvidersResponse["providers"];
};

export const ProvidersView: React.FC<ProvidersViewProps> = ({
  configuredProviders,
}) => {
  const dispatch = useAppDispatch();

  const currentHost = useAppSelector(selectConfig).host;
  const globalError = useAppSelector(getErrorMessage);
  const information = useAppSelector(getInformationMessage);

  const [currentProvider, setCurrentProvider] = useState<SimplifiedProvider<
    "name" | "enabled" | "readonly" | "supports_completion"
  > | null>(null);
  const handleSetCurrentProvider = useCallback(
    (
      provider: SimplifiedProvider<
        "name" | "enabled" | "readonly" | "supports_completion"
      > | null,
    ) => {
      setCurrentProvider(provider);
    },
    [],
  );

  return (
    <Flex px="1" direction="column" height="100%" width="100%">
      {!currentProvider && (
        <ConfiguredProvidersView
          configuredProviders={configuredProviders}
          handleSetCurrentProvider={handleSetCurrentProvider}
        />
      )}
      {currentProvider && (
        <ProviderPreview
          currentProvider={currentProvider}
          configuredProviders={configuredProviders}
          handleSetCurrentProvider={handleSetCurrentProvider}
        />
      )}
      {information && (
        <InformationCallout
          timeout={3000}
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
