import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { RootState } from "../../app/store";
import { DOCKER_CONTAINER_ACTION, DOCKER_CONTAINER_LIST } from "./consts";
import { callEngine } from "./call_engine";

export const dockerApi = createApi({
  reducerPath: "dockerApi",
  tagTypes: ["DOCKER"],
  baseQuery: fetchBaseQuery({
    prepareHeaders: (headers, api) => {
      const getState = api.getState as () => RootState;
      const state = getState();
      const token = state.config.apiKey;
      headers.set("credentials", "same-origin");
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  endpoints: (builder) => ({
    getAllDockerContainers: builder.query<DockerContainersResponse, undefined>({
      providesTags: ["DOCKER"],
      async queryFn(_arg, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, DOCKER_CONTAINER_LIST, {
            method: "POST",
            body: JSON.stringify({}),
            credentials: "same-origin",
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isDockerContainersResponse(data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse docker containers response",
                data: data,
              },
            };
          }
          return { data };
        } catch (error) {
          return {
            error: {
              status: "FETCH_ERROR",
              error: String(error),
            },
          };
        }
      },
    }),
    getDockerContainersByLabel: builder.query<DockerContainersResponse, string>({
      providesTags: ["DOCKER"],
      async queryFn(label, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, DOCKER_CONTAINER_LIST, {
            method: "POST",
            body: JSON.stringify({ label }),
            credentials: "same-origin",
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isDockerContainersResponse(data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse docker containers by labels response",
                data: data,
              },
            };
          }
          return { data };
        } catch (error) {
          return {
            error: {
              status: "FETCH_ERROR",
              error: String(error),
            },
          };
        }
      },
    }),
    getDockerContainersByImage: builder.query<DockerContainersResponse, string>({
      providesTags: ["DOCKER"],
      async queryFn(image, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, DOCKER_CONTAINER_LIST, {
            method: "POST",
            body: JSON.stringify({ image }),
            credentials: "same-origin",
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isDockerContainersResponse(data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse docker containers by images response",
                data: data,
              },
            };
          }
          return { data };
        } catch (error) {
          return {
            error: {
              status: "FETCH_ERROR",
              error: String(error),
            },
          };
        }
      },
    }),
    getDockerContainersByImageAndLabel: builder.query<
      DockerContainersResponse,
      DockerRequestBody
    >({
      providesTags: ["DOCKER"],
      async queryFn(args, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, DOCKER_CONTAINER_LIST, {
            method: "POST",
            body: JSON.stringify({
              image: args.image,
              label: args.label,
            }),
            credentials: "same-origin",
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isDockerContainersResponse(data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse docker containers by images response",
                data: data,
              },
            };
          }
          return { data };
        } catch (error) {
          return {
            error: {
              status: "FETCH_ERROR",
              error: String(error),
            },
          };
        }
      },
    }),
    executeActionForDockerContainer: builder.mutation<
      DockerActionResponse,
      DockerActionPayload
    >({
      async queryFn(args, api, _extraOptions, _baseQuery) {
        try {
          const state = api.getState() as RootState;
          const data = await callEngine<unknown>(state, DOCKER_CONTAINER_ACTION, {
            method: "POST",
            body: JSON.stringify({
              action: args.action,
              container: args.container,
            }),
            credentials: "same-origin",
            headers: {
              "Content-Type": "application/json",
            },
          });

          if (!isDockerActionResponse(data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: `Failed to execute ${args.action} for docker container with ${args.container} name/id!`,
                data: data,
              },
            };
          }
          return { data };
        } catch (error) {
          return {
            error: {
              status: "FETCH_ERROR",
              error: String(error),
            },
          };
        }
      },
    }),
  }),
});

export type DockerActionResponse = {
  success: boolean;
  output: string;
};

export type DockerActionPayload = {
  action: "start" | "stop" | "kill" | "remove";
  container: string;
};

function isDockerActionResponse(json: unknown): json is DockerActionResponse {
  if (!json) return false;
  if (typeof json !== "object") return false;
  if (!("success" in json)) return false;
  if (typeof json.success !== "boolean") return false;
  if (!("output" in json)) return false;
  if (typeof json.output !== "string") return false;
  return true;
}

type DockerRequestBody = {
  label?: string;
  image?: string;
};

export type DockerContainer = {
  id: string;
  name: string;
  status: string;
  created: string;
  user: string;
  env: string[];
  command: string[];
  image: string;
  working_dir: string;
  labels: DockerLabels;
  ports: DockerPorts;
};

type DockerLabels = NonNullable<unknown>;
type DockerPorts = NonNullable<unknown>;

type DockerContainersResponse = {
  containers: DockerContainer[];
  has_connection_to_docker_daemon: boolean;
  docker_error?: string;
};

function isDockerContainersResponse(
  json: unknown,
): json is DockerContainersResponse {
  if (
    !json ||
    typeof json !== "object" ||
    !Array.isArray((json as DockerContainersResponse).containers)
  ) {
    return false;
  }
  const containers = (json as DockerContainersResponse).containers;
  if (!containers.every(isDockerContainer)) {
    return false;
  }

  if (
    "has_connection_to_docker_daemon" in json &&
    typeof json.has_connection_to_docker_daemon !== "boolean"
  ) {
    return false;
  }

  if ("docker_error" in json && typeof json.docker_error !== "string") {
    return false;
  }
  return true;
}

function isDockerContainer(json: unknown): json is DockerContainer {
  if (!json || typeof json !== "object") return false;

  const container = json as DockerContainer;

  if (typeof container.id !== "string") return false;
  if (typeof container.name !== "string") return false;
  if (typeof container.status !== "string") return false;
  if (typeof container.created !== "string") return false;
  if (typeof container.user !== "string") return false;
  if (
    !Array.isArray(container.env) ||
    !container.env.every((e) => typeof e === "string")
  )
    return false;

  if (
    !Array.isArray(container.command) ||
    !container.command.every((c) => typeof c === "string")
  )
    return false;

  if (typeof container.image !== "string") return false;
  if (typeof container.working_dir !== "string") return false;
  if (!isDockerLabels(container.labels)) return false;
  if (!isDockerPorts(container.ports)) return false;
  return true;
}

function isDockerLabels(json: unknown): json is DockerLabels {
  return json !== null && json !== undefined;
}

function isDockerPorts(json: unknown): json is DockerPorts {
  return json !== null && json !== undefined;
}

export function jsonHasWhenIsolated(
  json: unknown,
): json is Record<string, boolean> & { when_isolated: boolean } {
  return (
    typeof json === "object" &&
    json !== null &&
    "when_isolated" in json &&
    typeof json.when_isolated === "boolean"
  );
}