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
import { Button, Card, Flex, Heading, Text } from "@radix-ui/themes";
import { DockerContainerCard } from "./DockerContainerCard";
import { SmartLink } from "../../SmartLink";

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
      <DockerErrorCard
        errorType="no-containers"
        integrationPath={integrationPath}
        handleSwitchIntegration={handleSwitchIntegration}
      />
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
  <>
    <Heading as="h6" size="3" weight="bold" align="center">
      No connection to Docker Daemon
    </Heading>
    <Text size="2">
      Seems, that there is no connection to Docker Daemon. Please, setup Docker
      properly or check if Docker Engine is running
    </Text>
    <Button
      variant="outline"
      color="gray"
      onClick={() => handleSwitchIntegration("docker", integrationPath)}
    >
      Setup docker
    </Button>
  </>
);

const UnexpectedError: FC = () => (
  <>
    <Heading as="h6" size="3" weight="bold" align="center">
      Unexpected error
    </Heading>
    <Text size="2">
      Something went wrong during connection or listing containers
    </Text>
  </>
);

const NoContainersError: FC = () => (
  <>
    <Heading as="h6" size="3" weight="bold" align="center">
      No containers
    </Heading>
    <Text size="2">
      No Docker containers found. Please, ensure that containers are running.
    </Text>
  </>
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
        <ErrorComponent
          handleSwitchIntegration={handleSwitchIntegration}
          integrationPath={integrationPath}
        />
      </Flex>
    </Card>
  );
};
/*


{
  "project_path": "",
  "integr_name": "postgres",
  "integr_config_path": "C:\\Users\\andre\\.config/refact\\integrations.d\\postgres.yaml",
  "integr_schema": {
    "fields": {
      "host": {
        "f_type": "string_long",
        "f_desc": "Connect to this host, for example 127.0.0.1 or docker container name.",
        "f_placeholder": "marketing_db_container"
      },
      "port": {
        "f_type": "string_short",
        "f_desc": "Which port to use.",
        "f_default": "5432"
      },
      "user": {
        "f_type": "string_short",
        "f_placeholder": "john_doe"
      },
      "password": {
        "f_type": "string_short",
        "f_default": "$POSTGRES_PASSWORD",
        "smartlinks": [
          {
            "sl_label": "Open passwords.yaml",
            "sl_goto": "EDITOR:passwords.yaml"
          }
        ]
      },
      "database": {
        "f_type": "string_short",
        "f_placeholder": "marketing_db"
      },
      "psql_binary_path": {
        "f_type": "string_long",
        "f_desc": "If it can't find a path to `psql` you can provide it here, leave blank if not sure.",
        "f_placeholder": "psql",
        "f_label": "PSQL Binary Path",
        "f_extra": true
      }
    },
    "description": "The Postgres tool is for the AI model to call, when it wants to look at data inside your database, or make any changes.\nOn this page you can also see Docker containers with Postgres servers.\nYou can ask model to create a new container with a new database for you,\nor ask model to configure the tool to use an existing container with existing database.\n",
    "available": {
      "on_your_laptop_possible": true,
      "when_isolated_possible": true
    },
    "smartlinks": [
      {
        "sl_label": "Test",
        "sl_chat": [
          {
            "role": "user",
            "content": "🔧 The postgres tool should be visible now. To test the tool, list the tables available, briefly describe the tables and express\nhappiness, and change nothing. If it doesn't work or the tool isn't available, go through the usual plan in the system prompt.\nThe current config file is %CURRENT_CONFIG%.\n"
          }
        ]
      },
      {
        "sl_label": "Look at the project, fill in automatically",
        "sl_chat": [
          {
            "role": "user",
            "content": "🔧 Your goal is to set up postgres client. Look at the project, especially files like \"docker-compose.yaml\" or \".env\". Call tree() to see what files the project has.\nAfter that is completed, go through the usual plan in the system prompt.\nThe current config file is %CURRENT_CONFIG%.\n"
          }
        ]
      }
    ],
    "docker": {
      "filter_label": "",
      "filter_image": "postgres",
      "new_container_default": {
        "image": "postgres:13",
        "environment": {
          "POSTGRES_DB": "marketing_db",
          "POSTGRES_USER": "john_doe",
          "POSTGRES_PASSWORD": "$POSTGRES_PASSWORD"
        }
      },
      "smartlinks": [
        {
          "sl_label": "Add Database Container",
          "sl_chat": [
            {
              "role": "user",
              "content": "🔧 Your job is to create a postgres container, using the image and environment from new_container_default section in the current config file: %CURRENT_CONFIG%. Follow the system prompt.\n"
            }
          ]
        }
      ],
      "smartlinks_for_each_container": [
        {
          "sl_label": "Use for integration",
          "sl_chat": [
            {
              "role": "user",
              "content": "🔧 Your job is to modify postgres connection config in the current file to match the variables from the container, use docker tool to inspect the container if needed. Current config file: %CURRENT_CONFIG%.\n"
            }
          ]
        }
      ]
    }
  },
  "integr_values": {
    "psql_binary_path": "psql",
    "host": "127.0.0.1",
    "port": "5432",
    "user": "postgres",
    "password": "postgrespass",
    "database": "postgres",
    "available": {
      "on_your_laptop": true,
      "when_isolated": false
    }
  },
  "error_log": []
}



{
  "project_path": "",
  "integr_name": "cmdline_a_out_run",
  "integr_config_path": "C:\\Users\\andre\\.config/refact\\integrations.d\\cmdline_a_out_run.yaml",
  "integr_schema": {
    "fields": {
      "command": {
        "f_type": "string_long",
        "f_desc": "The command to execute.",
        "f_placeholder": "echo Hello World"
      },
      "command_workdir": {
        "f_type": "string_long",
        "f_desc": "The working directory for the command.",
        "f_placeholder": "/path/to/workdir"
      },
      "description": {
        "f_type": "string_long",
        "f_desc": "The model will see this description, why the model should call this?"
      },
      "parameters": {
        "f_type": "tool_parameters",
        "f_desc": "The model will fill in those parameters."
      },
      "timeout": {
        "f_type": "string_short",
        "f_desc": "The command must immediately return the results, it can't be interactive. If the command runs for too long, it will be terminated and stderr/stdout collected will be presented to the model.",
        "f_default": "10"
      },
      "output_filter": {
        "f_type": "output_filter",
        "f_desc": "The output from the command can be long or even quasi-infinite. This section allows to set limits, prioritize top or bottom, or use regexp to show the model the relevant part.",
        "f_placeholder": "filter"
      }
    },
    "description": "There you can adapt any command line tool for use by AI model. You can give the model instructions why to call it, which parameters to provide,\nset a timeout and restrict the output. If you want a tool that runs in the background such as a web server, use service_* instead.\n",
    "available": {
      "on_your_laptop_possible": true,
      "when_isolated_possible": true
    }
  },
  "integr_values": {
    "command": "",
    "command_workdir": "",
    "description": "",
    "parameters": [],
    "parameters_required": null,
    "timeout": "",
    "output_filter": {
      "limit_lines": 100,
      "limit_chars": 10000,
      "valuable_top_or_bottom": "top",
      "grep": "(?i)error",
      "grep_context_lines": 5,
      "remove_from_output": ""
    },
    "startup_wait_port": null,
    "startup_wait": 0,
    "startup_wait_keyword": null,
    "available": {
      "on_your_laptop": false,
      "when_isolated": false
    }
  },
  "error_log": []
}


*/
