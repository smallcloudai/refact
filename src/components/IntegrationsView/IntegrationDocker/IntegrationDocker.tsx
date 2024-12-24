import type { FC } from "react";
import { useEffect, useState } from "react";
import { useGetDockerContainersByImageQuery } from "../../../hooks/useGetDockerContainersQuery";
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
import { Card, Flex, Heading, Text } from "@radix-ui/themes";
import { DockerContainerCard } from "./DockerContainerCard";
import { SmartLink } from "../../SmartLink";
import { Link } from "../../Link";

type IntegrationDockerProps = {
  dockerData: SchemaDocker;
  integrationName: string;
  integrationPath: string;
  integrationProject: string;
  handleSwitchIntegration: (
    integrationName: string,
    integrationConfigPath: string,
  ) => void;
};

export const IntegrationDocker: FC<IntegrationDockerProps> = ({
  dockerData,
  integrationName,
  integrationPath,
  integrationProject,
  handleSwitchIntegration,
}) => {
  const dispatch = useAppDispatch();
  const { dockerContainersResponse } = useGetDockerContainersByImageQuery(
    dockerData.filter_image,
  );
  const [areContainersLoaded, setAreContainersLoaded] = useState(false);

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
    if (!dockerContainersResponse.isLoading) {
      if (dockerContainersResponse.data) {
        setDockerContainersList(dockerContainersResponse.data.containers);
      }
      timeoutId = setTimeout(() => {
        setAreContainersLoaded(true);
      }, 100);
    }

    return () => {
      clearTimeout(timeoutId);
    };
  }, [dockerContainersResponse, areContainersLoaded]);

  if (dockerContainersResponse.isLoading || !areContainersLoaded) {
    return <Spinner spinning />;
  }

  if (
    !dockerContainersResponse.data ||
    !dockerContainersResponse.data.has_connection_to_docker_daemon
  ) {
    return (
      <DockerErrorCard
        errorType="no-connection"
        integrationPath={integrationPath}
        handleSwitchIntegration={handleSwitchIntegration}
      />
    );
  }

  if (!dockerContainersList || dockerContainersList.length === 0) {
    return (
      <>
        <DockerErrorCard
          errorType="no-containers"
          integrationPath={integrationPath}
          handleSwitchIntegration={handleSwitchIntegration}
        />
        {/* TODO: duplicative code */}
        {dockerData.smartlinks.length > 0 && (
          <Flex direction="column" width="100%" gap="3" mt="2">
            <Heading size="4" as="h4">
              Ask AI to do it for you (experimental)
            </Heading>
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
        )}
      </>
    );
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
      <Flex direction="column" width="100%" gap="3" mt="2">
        <Heading size="4" as="h4">
          Ask AI to do it for you (experimental)
        </Heading>
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
    </Flex>
  );
};

type DockerErrorCardProps = {
  errorType: "no-connection" | "unexpected" | "no-containers";
  handleSwitchIntegration: (
    integrationName: string,
    integrationConfigPath: string,
  ) => void;
  integrationPath: string;
};

const NoConnectionError: FC<{
  handleSwitchIntegration: (
    integrationName: string,
    integrationConfigPath: string,
  ) => void;
  integrationPath: string;
}> = ({ handleSwitchIntegration, integrationPath }) => (
  <Text size="2" color="gray">
    Seems, like there is an issue with connection to your Docker Daemon.
    Consider reviewing your{" "}
    <Link
      color="indigo"
      onClick={() => handleSwitchIntegration("docker", integrationPath)}
    >
      Docker Configuration
    </Link>{" "}
    or check if the Docker Engine is running.
  </Text>
);

const UnexpectedError: FC = () => (
  <Card
    style={{
      margin: "1rem auto 0",
      width: "100%",
    }}
  >
    <Flex
      direction="column"
      align="stretch"
      justify="center"
      gap="4"
      width="100%"
    >
      <Heading as="h6" size="3" weight="bold" align="center">
        Unexpected error
      </Heading>
      <Text size="2">
        Something went wrong during connection or listing containers
      </Text>
    </Flex>
  </Card>
);

const NoContainersError: FC = () => (
  <Card
    style={{
      margin: "1rem auto 0",
      width: "100%",
    }}
  >
    <Flex
      direction="column"
      align="stretch"
      justify="center"
      gap="4"
      width="100%"
    >
      <Heading as="h6" size="3" weight="bold" align="center">
        No containers
      </Heading>
      <Text size="2">
        No Docker containers found. Please, ensure that containers are running.
      </Text>
    </Flex>
  </Card>
);

const errorComponents = {
  "no-connection": NoConnectionError,
  unexpected: UnexpectedError,
  "no-containers": NoContainersError,
};

const DockerErrorCard: FC<DockerErrorCardProps> = ({
  errorType,
  integrationPath,
  handleSwitchIntegration,
}) => {
  const ErrorComponent = errorComponents[errorType];
  return (
    <ErrorComponent
      handleSwitchIntegration={handleSwitchIntegration}
      integrationPath={integrationPath}
    />
  );
};
