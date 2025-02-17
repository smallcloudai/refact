import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { RootState } from "../../app/store";
import { DOCKER_CONTAINER_ACTION, DOCKER_CONTAINER_LIST } from "./consts";

// TODO: There might be some cache issues here

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
      // TODO: make a function for settings tags
      providesTags: ["DOCKER"],
      async queryFn(_arg, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${DOCKER_CONTAINER_LIST}`;
        const response = await baseQuery({
          url,
          // LSP cannot handle regular GET request :/
          method: "POST",
          body: {},
          ...extraOptions,
        });

        if (response.error) {
          return { error: response.error };
        }

        if (!isDockerContainersResponse(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: "Failed to parse docker containers response",
              data: response.data,
            },
          };
        }
        return { data: response.data };
      },
    }),
    getDockerContainersByLabel: builder.query<DockerContainersResponse, string>(
      {
        providesTags: ["DOCKER"],
        async queryFn(label, api, extraOptions, baseQuery) {
          const state = api.getState() as RootState;
          const port = state.config.lspPort as unknown as number;
          const url = `http://127.0.0.1:${port}${DOCKER_CONTAINER_LIST}`;
          const response = await baseQuery({
            url,
            method: "POST",
            body: {
              label,
            },
            ...extraOptions,
          });

          if (response.error) {
            return { error: response.error };
          }

          if (!isDockerContainersResponse(response.data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse docker containers by labels response",
                data: response.data,
              },
            };
          }
          return { data: response.data };
        },
      },
    ),
    getDockerContainersByImage: builder.query<DockerContainersResponse, string>(
      {
        providesTags: ["DOCKER"],
        async queryFn(image, api, extraOptions, baseQuery) {
          const state = api.getState() as RootState;
          const port = state.config.lspPort as unknown as number;
          const url = `http://127.0.0.1:${port}${DOCKER_CONTAINER_LIST}`;
          const response = await baseQuery({
            url,
            method: "POST",
            body: {
              image,
            },
            ...extraOptions,
          });

          if (response.error) {
            return { error: response.error };
          }

          if (!isDockerContainersResponse(response.data)) {
            return {
              error: {
                status: "CUSTOM_ERROR",
                error: "Failed to parse docker containers by images response",
                data: response.data,
              },
            };
          }
          return { data: response.data };
        },
      },
    ),
    getDockerContainersByImageAndLabel: builder.query<
      DockerContainersResponse,
      DockerRequestBody
    >({
      providesTags: ["DOCKER"],
      async queryFn(args, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${DOCKER_CONTAINER_LIST}`;
        const response = await baseQuery({
          url,
          method: "POST",
          body: {
            image: args.image,
            label: args.label,
          },
          ...extraOptions,
        });

        if (response.error) {
          return { error: response.error };
        }

        if (!isDockerContainersResponse(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: "Failed to parse docker containers by images response",
              data: response.data,
            },
          };
        }
        return { data: response.data };
      },
    }),
    executeActionForDockerContainer: builder.mutation<
      DockerActionResponse,
      DockerActionPayload
    >({
      async queryFn(args, api, extraOptions, baseQuery) {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${DOCKER_CONTAINER_ACTION}`;
        const response = await baseQuery({
          url,
          method: "POST",
          body: {
            action: args.action,
            container: args.container,
          },
          ...extraOptions,
        });

        if (response.error) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: `Failed to execute ${args.action} for docker container with ${args.container} name/id!`,
              data: response.error.data,
            },
          };
        }

        if (!isDockerActionResponse(response.data)) {
          return {
            error: {
              status: "CUSTOM_ERROR",
              error: `Failed to execute ${args.action} for docker container with ${args.container} name/id!`,
              data: response.data,
            },
          };
        }
        return { data: response.data };
      },
    }),
  }),
});

export type DockerActionResponse = {
  success: boolean;
  output: string;
};

/**
 * Represents the payload for a Docker action endpoints.
 * @param action Docker action for a specific operation (start, stop, kill, remove)
 * @param container This can be either the container name or the container ID.
 */
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

// TODO: make types for ports
type DockerPorts = NonNullable<unknown>;

// TODO: make type guards better
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
  // Since DockerPorts is defined as NonNullable<unknown>, we don't have specific structure to validate. Just checking, that it's not null | undefined
  return json !== null && json !== undefined;
}

function isDockerPorts(json: unknown): json is DockerPorts {
  // Since DockerPorts is defined as NonNullable<unknown>, we don't have specific structure to validate. Just checking, that it's not null | undefined
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
