import type { FC } from "react";
import { Flex, Heading, Separator, Text } from "@radix-ui/themes";

import type { ProviderFormProps } from "../ProviderForm";

import { Spinner } from "../../../../components/Spinner";
import { ModelCard } from "./ModelCard";
import { AddModelButton } from "./components";

import { useGetModelsByProviderNameQuery } from "../../../../hooks/useModelsQuery";

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

  if (isLoading) return <Spinner spinning />;

  if (!isSuccess) return <div>Something went wrong :/</div>;

  const { chat_models, completion_models } = modelsData;

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
              key={m.name}
              model={m}
              providerName={provider.name}
              modelType="chat"
              isReadonlyProvider={provider.readonly}
            />
          );
        })
      ) : (
        <NoModelsText />
      )}
      {!provider.readonly && (
        <AddModelButton modelType="chat" providerName={provider.name} />
      )}
      <Heading as="h6" size="2" my="2">
        Completion Models
      </Heading>
      {completion_models.length > 0 ? (
        completion_models.map((m) => {
          return (
            <ModelCard
              key={m.name}
              model={m}
              providerName={provider.name}
              modelType="completion"
              isReadonlyProvider={provider.readonly}
            />
          );
        })
      ) : (
        <NoModelsText />
      )}
      {!provider.readonly && (
        <AddModelButton modelType="completion" providerName={provider.name} />
      )}
      {/* TODO: do we want to expose embedding model configuration updates? */}
      {/* <Heading as="h6" size="2">
        Embedding Model
      </Heading>
      <div>{modelsData.embedding_model.name}</div> */}
    </Flex>
  );
};
