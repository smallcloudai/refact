import type { FC } from "react";
import { useEffect, useState } from "react";
import {
  useGetDockerContainersByImageQuery,
  // useGetDockerContainersQuery,
} from "../../../hooks/useGetDockerContainersQuery";
import { dockerApi } from "../../../services/refact";
import type {
  DockerActionResponse,
  DockerActionPayload,
  DockerContainer,
  SchemaDocker,
} from "../../../services/refact";
import { Spinner } from "../../Spinner";
import { useExecuteActionForDockerContainerMutation } from "../../../hooks/useExecuteActionForDockerContainer";
import { useAppDispatch } from "../../../hooks";
import { setInformation } from "../../../features/Errors/informationSlice";
import { setError } from "../../../features/Errors/errorsSlice";
import { Card, Flex, Text } from "@radix-ui/themes";
import { DockerContainerCard } from "./DockerContainerCard";
import { SmartLink } from "../../SmartLink";

type IntegrationDockerProps = {
  dockerData: SchemaDocker;
  integrationName: string;
  integrationPath: string;
  integrationProject: string;
};

export const IntegrationDocker: FC<IntegrationDockerProps> = ({
  dockerData,
  integrationName,
  integrationPath,
  integrationProject,
}) => {
  const dispatch = useAppDispatch();
  const { dockerContainers } = useGetDockerContainersByImageQuery(
    dockerData.filter_image,
  );
  const [areContainersLoaded, setAreContainersLoaded] = useState(false);

  // const { dockerContainers } = useGetDockerContainersQuery();
  const [dockerContainerActionTrigger] =
    useExecuteActionForDockerContainerMutation();
  const [isActionInProgress, setIsActionInProgress] = useState(false);
  const [currentContainerAction, setCurrentContainerAction] =
    useState<DockerActionPayload | null>(null);

  const [dockerContainersList, setDockerContainersList] = useState<
    DockerContainer[] | null
  >(null);

  useEffect(() => {
    let timeoutId: NodeJS.Timeout;
    if (!dockerContainers.isLoading) {
      if (dockerContainers.data) {
        setDockerContainersList(dockerContainers.data.containers);
      }
      timeoutId = setTimeout(() => {
        setAreContainersLoaded(true);
      }, 100);
    }

    return () => {
      clearTimeout(timeoutId);
    };
  }, [dockerContainers, areContainersLoaded]);

  if (dockerContainers.isLoading || !areContainersLoaded) {
    return <Spinner spinning />;
  }

  if (dockerContainers.error ?? !dockerContainers.data) {
    return <DockerErrorCard errorType="no-connection" />;
  }

  if (!dockerContainersList || dockerContainersList.length === 0) {
    return <DockerErrorCard errorType="unexpected" />;
  }

  const handleDockerContainerActionClick = async (
    payload: DockerActionPayload,
  ) => {
    setIsActionInProgress(true);
    setCurrentContainerAction(payload);

    const response = await dockerContainerActionTrigger({
      container: payload.container,
      action: payload.action,
    });

    if (response.error) {
      resetActionState();
      return;
    }

    handleResponse(response.data, payload);
    resetActionState();
  };

  const resetActionState = () => {
    setIsActionInProgress(false);
    setCurrentContainerAction(null);
  };

  const handleResponse = (
    data: DockerActionResponse,
    payload: DockerActionPayload,
  ) => {
    if (data.success) {
      dispatch(
        setInformation(
          `Action ${payload.action} was successfully executed on ${payload.container} container`,
        ),
      );
      dispatch(dockerApi.util.resetApiState());
    } else {
      dispatch(
        setError(
          `Action ${payload.action} failed to execute on ${payload.container} container`,
        ),
      );
    }
  };

  return (
    <Flex direction="column" gap="4" width="100%">
      <Flex direction="column" gap="2">
        {dockerContainersList.map((el) => (
          <DockerContainerCard
            key={el.id}
            container={el}
            currentContainerAction={currentContainerAction}
            isActionInProgress={isActionInProgress}
            handleDockerContainerActionClick={handleDockerContainerActionClick}
            integrationData={{
              integrationName,
              integrationPath,
              integrationProject,
            }}
            containerSmartlinks={dockerData.smartlinks_for_each_container}
          />
        ))}
      </Flex>
      <Flex gap="2" align="center">
        {dockerData.smartlinks.map((smartlink) => (
          <SmartLink
            key={`docker-container-${dockerData.filter_image}`}
            integrationName={integrationName}
            integrationPath={integrationPath}
            integrationProject={integrationProject}
            smartlink={smartlink}
          />
        ))}
      </Flex>
    </Flex>
  );
};

type DockerErrorCardProps = {
  errorType: "no-connection" | "unexpected";
};

const DockerErrorCard: FC<DockerErrorCardProps> = ({ errorType }) => {
  return (
    <Card
      style={{
        margin: "1rem auto 0",
      }}
    >
      <Flex
        direction="column"
        align="center"
        justify="center"
        gap="4"
        width="100%"
      >
        <Text size="3" weight="bold">
          {errorType === "no-connection" ? "No connection" : "Unexpected error"}
        </Text>
        <Text size="2">
          {errorType === "no-connection"
            ? "Seems, that there is no connection to Docker Daemon. Please, setup Docker properly or launch Docker Engine"
            : "Something went wrong during connection or listing containers"}
        </Text>
      </Flex>
    </Card>
  );
};
