import { useCallback, useMemo, type FC } from "react";
import { Flex, Heading, Separator, Text } from "@radix-ui/themes";

import type { ProviderFormProps } from "../ProviderForm";

import { Spinner } from "../../../../components/Spinner";
import { ModelCard } from "./ModelCard";
import { AddModelButton } from "./components";

import { useGetModelsByProviderNameQuery } from "../../../../hooks/useModelsQuery";
import { ModelsResponse, useGetCapsQuery } from "../../../../services/refact";
import { groupModelsWithPricing } from "./utils/groupModelsWithPricing";

export type ProviderModelsListProps = {
  provider: ProviderFormProps["currentProvider"];
};

const NoModelsText: FC = () => {
  return (
    <Text as="span" size="2" color="gray">
      No models available, but you can add one by clicking &apos;Add model&apos;
    </Text>
  );
};

export const ProviderModelsList: FC<ProviderModelsListProps> = ({
  provider,
}) => {
  const {
    data: modelsData,
    isSuccess,
    isLoading,
  } = useGetModelsByProviderNameQuery({
    providerName: provider.name,
  });

  // Fetch capabilities & pricing; UI will gracefully degrade if this fails.
  const { data: capsData, isError: capsError } = useGetCapsQuery(undefined);

  const getModelNames = useCallback((modelsData: ModelsResponse) => {
    const currentChatModelNames = modelsData.chat_models.map((m) => m.name);
    const currentCompletionModelNames = modelsData.completion_models.map(
      (m) => m.name,
    );

    return {
      currentChatModelNames,
      currentCompletionModelNames,
    };
  }, []);

  // Compute groups early so hooks are always called in the same order
  const chatGroups = useMemo(
    () =>
      modelsData?.chat_models
        ? groupModelsWithPricing(modelsData.chat_models, {
            caps: capsError ? undefined : capsData,
            modelType: "chat",
          })
        : [],
    [modelsData?.chat_models, capsData, capsError],
  );

  const completionGroups = useMemo(
    () =>
      modelsData?.completion_models
        ? groupModelsWithPricing(modelsData.completion_models, {
            caps: capsError ? undefined : capsData,
            modelType: "completion",
          })
        : [],
    [modelsData?.completion_models, capsData, capsError],
  );

  if (isLoading) return <Spinner spinning />;

  if (!isSuccess) return <div>Something went wrong :/</div>;

  const { chat_models, completion_models } = modelsData;

  const { currentChatModelNames, currentCompletionModelNames } =
    getModelNames(modelsData);

  return (
    <Flex direction="column" gap="2">
      <Heading as="h3" size="3">
        Models list
      </Heading>
      <Separator size="4" />

      {/* Chat models section */}
      <Heading as="h6" size="2" my="2">
        Chat Models
      </Heading>

      {chat_models.length > 0 ? (
        chatGroups.map((group) => (
          <Flex key={group.id} direction="column" gap="1" my="1">
            {chatGroups.length > 1 && (
              <Text as="span" size="1" color="gray" weight="medium">
                {group.title}
                {group.description ? ` — ${group.description}` : ""}
              </Text>
            )}
            {group.models.map((m) => (
              <ModelCard
                key={`${m.name}_chat`}
                model={m}
                providerName={provider.name}
                modelType="chat"
                isReadonlyProvider={provider.readonly}
                currentModelNames={currentChatModelNames}
              />
            ))}
          </Flex>
        ))
      ) : (
        <NoModelsText />
      )}

      {!provider.readonly && (
        <AddModelButton
          modelType="chat"
          providerName={provider.name}
          currentModelNames={currentChatModelNames}
        />
      )}

      {/* Completion models section */}
      {provider.supports_completion && (
        <>
          <Heading as="h6" size="2" my="2">
            Completion Models
          </Heading>
          {completion_models.length > 0 ? (
            completionGroups.map((group) => (
              <Flex key={group.id} direction="column" gap="1" my="1">
                {completionGroups.length > 1 && (
                  <Text as="span" size="1" color="gray" weight="medium">
                    {group.title}
                    {group.description ? ` — ${group.description}` : ""}
                  </Text>
                )}
                {group.models.map((m) => (
                  <ModelCard
                    key={`${m.name}_completion`}
                    model={m}
                    providerName={provider.name}
                    modelType="completion"
                    isReadonlyProvider={provider.readonly}
                    currentModelNames={currentCompletionModelNames}
                  />
                ))}
              </Flex>
            ))
          ) : (
            <NoModelsText />
          )}

          {!provider.readonly && (
            <AddModelButton
              modelType="completion"
              providerName={provider.name}
              currentModelNames={currentCompletionModelNames}
            />
          )}
        </>
      )}

      {/* Embedding model could be handled in a similar way in the future */}
    </Flex>
  );
};
