import { useCallback, useMemo, useState } from "react";
import {
  areAllFieldsBoolean,
  integrationsApi,
  IntegrationWithIconRecord,
  NotConfiguredIntegrationWithIconRecord,
} from "../../../services/refact";
import { setError } from "../../../features/Errors/errorsSlice";
import { useAppDispatch } from "../../../hooks";

export const useUpdateIntegration = ({
  integration,
}: {
  integration:
    | IntegrationWithIconRecord
    | NotConfiguredIntegrationWithIconRecord;
}) => {
  const dispatch = useAppDispatch();

  const [getIntegrationData] =
    integrationsApi.useLazyGetIntegrationByPathQuery();
  const [saveIntegrationData] = integrationsApi.useSaveIntegrationMutation();
  const [updatedAvailability, setUpdatedAvailability] = useState<
    Record<string, boolean>
  >({
    on_your_laptop: integration.on_your_laptop,
    when_isolated: integration.when_isolated,
  });

  const [isUpdatingAvailability, setIsUpdatingAvailability] = useState(false);

  const updateIntegrationAvailability = useCallback(async () => {
    if (Array.isArray(integration.integr_config_path)) {
      return;
    }

    setIsUpdatingAvailability(true);

    const { data: integrationData } = await getIntegrationData(
      integration.integr_config_path,
    );

    if (!integrationData?.integr_values) {
      return;
    }

    const { available } = integrationData.integr_values;
    const newAvailability = areAllFieldsBoolean(available)
      ? {
          on_your_laptop: !available.on_your_laptop,
          when_isolated: available.when_isolated,
        }
      : {
          on_your_laptop: integration.on_your_laptop,
          when_isolated: integration.when_isolated,
        };

    const response = await saveIntegrationData({
      filePath: integration.integr_config_path,
      values: {
        ...integrationData.integr_values,
        available: newAvailability,
      },
    });
    if (response.error) {
      dispatch(
        setError(
          `Error occurred on updating ${integration.integr_name} configuration. Check if your integration configuration is correct`,
        ),
      );
      setIsUpdatingAvailability(false);
      return;
    }

    setUpdatedAvailability(newAvailability);
    setIsUpdatingAvailability(false);
  }, [dispatch, getIntegrationData, saveIntegrationData, integration]);

  const integrationAvailability = useMemo(() => {
    return updatedAvailability;
  }, [updatedAvailability]);

  return {
    updateIntegrationAvailability,
    integrationAvailability,
    isUpdatingAvailability,
  };
};
