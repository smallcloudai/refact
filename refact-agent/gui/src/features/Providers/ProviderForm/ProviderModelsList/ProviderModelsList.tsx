import { useCallback, type FC } from "react";
import { Flex, Heading, Separator, Text } from "@radix-ui/themes";

import type { ProviderFormProps } from "../ProviderForm";

import { Spinner } from "../../../../components/Spinner";
import { ModelCard } from "./ModelCard";
import { AddModelButton } from "./components";

import { useGetModelsByProviderNameQuery } from "../../../../hooks/useModelsQuery";
import { ModelsResponse } from "../../../../services/refact";

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
      <Heading as="h6" size="2" my="2">
        Chat Models
      </Heading>
      {chat_models.length > 0 ? (
        chat_models.map((m) => {
          return (
            <ModelCard
              key={`${m.name}_chat`}
              model={m}
              providerName={provider.name}
              modelType="chat"
              isReadonlyProvider={provider.readonly}
              currentModelNames={currentChatModelNames}
            />
          );
        })
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
      {provider.supports_completion && (
        <>
          <Heading as="h6" size="2" my="2">
            Completion Models
          </Heading>
          {completion_models.length > 0 ? (
            completion_models.map((m) => {
              return (
                <ModelCard
                  key={`${m.name}_completion`}
                  model={m}
                  providerName={provider.name}
                  modelType="completion"
                  isReadonlyProvider={provider.readonly}
                  currentModelNames={currentCompletionModelNames}
                />
              );
            })
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
      {/* TODO: do we want to expose embedding model configuration updates? */}
      {/* <Heading as="h6" size="2">
        Embedding Model
      </Heading>
      <div>{modelsData.embedding_model.name}</div> */}
    </Flex>
  );
};
