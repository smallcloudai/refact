import { useCallback, useMemo, useState } from "react";
import {
  areAllFieldsBoolean,
  integrationsApi,
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";
import { debugIntegrations } from "../../../debugConfig";

export const useUpdateIntegration = ({
  integration,
}: {
  integration:
    | IntegrationWithIconRecord
    | NotConfiguredIntegrationWithIconRecord;
}) => {
  const [getIntegrationData] =
    integrationsApi.useLazyGetIntegrationByPathQuery();
  const [saveIntegrationData] = integrationsApi.useSaveIntegrationMutation();
  const [updatedAvailability, setUpdatedAvailability] = useState<Record<
    string,
    boolean
  > | null>(null);
  const updateIntegrationAvailability = useCallback(async () => {
    if (!Array.isArray(integration.integr_config_path)) {
      const integrationData = await getIntegrationData(
        integration.integr_config_path,
      ).unwrap();
      const integrationValuesFromLSP = integrationData.integr_values;
      if (!integrationValuesFromLSP) {
        return;
      }
      debugIntegrations(
        `[DEBUG]: integrationValuesFromLSP: `,
        integrationValuesFromLSP,
      );
      const newAvailability = areAllFieldsBoolean(
        integrationValuesFromLSP.available,
      )
        ? {
            on_your_laptop: !integrationValuesFromLSP.available.on_your_laptop,
            when_isolated: integrationValuesFromLSP.available.when_isolated,
          }
        : {
            on_your_laptop: integration.on_your_laptop,
            when_isolated: integration.when_isolated,
          };

      const updatedIntegrationValues = {
        ...integrationValuesFromLSP,
        available: newAvailability,
      };

      debugIntegrations(
        `[DEBUG]: updated values to save: `,
        updatedIntegrationValues,
      );

      const response = await saveIntegrationData({
        filePath: integration.integr_config_path,
        values: updatedIntegrationValues,
      }).unwrap();

      setUpdatedAvailability(newAvailability);

      debugIntegrations(`[DEBUG]: response: `, response);

      // dispatch(integrationsApi.util.resetApiState());
    }
  }, [getIntegrationData, saveIntegrationData, integration]);

  const integrationAvailability = useMemo(() => {
    if (updatedAvailability) return updatedAvailability;
    return {
      on_your_laptop: integration.on_your_laptop,
      when_isolated: integration.when_isolated,
    };
  }, [updatedAvailability, integration]);

  return {
    updateIntegrationAvailability,
    integrationAvailability,
  };
};
