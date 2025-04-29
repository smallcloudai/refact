import { RootState } from "../../app/store";
import { hasProperty } from "../../utils";
import { isDetailMessage } from "./commands";
import {
  CONFIGURED_PROVIDERS_URL,
  PROVIDER_TEMPLATES_URL,
  PROVIDER_URL,
} from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";

export const providersApi = createApi({
  reducerPath: "providers",
  tagTypes: [
    "PROVIDERS",
    "TEMPLATE_PROVIDERS",
    "CONFIGURED_PROVIDERS",
    "PROVIDER",
  ],
  baseQuery: fetchBaseQuery({
    prepareHeaders: (headers, { getState }) => {
      const token = (getState() as RootState).config.apiKey;
      if (token) {
        headers.set("Authorization", `Bearer ${token}`);
      }
      return headers;
    },
  }),
  endpoints: (builder) => ({
    getConfiguredProviders: builder.query<
      ConfiguredProvidersResponse,
      undefined
    >({
      queryFn: async (_args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${CONFIGURED_PROVIDERS_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "GET",
          url,
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) {
          return { error: result.error };
        }
        if (!isConfiguredProvidersResponse(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/providers",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
      providesTags: [{ type: "CONFIGURED_PROVIDERS", id: "LIST" }],
    }),
    getProviderTemplates: builder.query<ProviderTemplatesResponse, undefined>({
      providesTags: ["TEMPLATE_PROVIDERS"],
      queryFn: async (_args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${PROVIDER_TEMPLATES_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "GET",
          url,
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) {
          return { error: result.error };
        }
        if (!isProviderTemplatesResponse(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/provider-templates",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
    getProvider: builder.query<Provider, { providerName: string }>({
      providesTags: ["PROVIDER"],
      queryFn: async (args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${PROVIDER_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "GET",
          url,
          params: {
            "provider-name": args.providerName,
          },
          credentials: "same-origin",
          redirect: "follow",
        });

        if (result.error) {
          return { error: result.error };
        }

        if (!isProvider(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/provider",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
    updateProvider: builder.mutation<unknown, Provider>({
      invalidatesTags: (_result, _error, args) => [
        { type: "PROVIDER", id: args.name },
      ],
      queryFn: async (args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${PROVIDER_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "POST",
          url,
          body: { ...args },
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) {
          return { error: result.error };
        }
        if (isDetailMessage(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/provider",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
    deleteProvider: builder.mutation<unknown, string>({
      invalidatesTags: (_result, _error, args) => [
        { type: "PROVIDER", id: args },
      ],
      queryFn: async (args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${PROVIDER_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "DELETE",
          url,
          params: {
            "provider-name": args,
          },
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) {
          return { error: result.error };
        }
        if (isDetailMessage(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/provider",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
  }),
  refetchOnMountOrArgChange: true,
});

export type Provider = {
  name: string;
  endpoint_style: "openai" | "hf";
  chat_endpoint: string;
  completion_endpoint: string;
  embedding_endpoint: string;
  api_key: string;

  chat_default_model: string;
  chat_thinking_model: string;
  chat_light_model: string;

  enabled: boolean;
  readonly: boolean;
  supports_completion?: boolean;
};

export type SimplifiedProvider<
  T extends keyof Provider | undefined = undefined,
> = [T] extends [undefined]
  ? Partial<Provider>
  : Required<Pick<Provider, T & keyof Provider>>;

export type ErrorLogInstance = {
  path: string;
  error_line: number;
  error_msg: string;
};

export type ConfiguredProvidersResponse = {
  providers: SimplifiedProvider<
    "name" | "enabled" | "readonly" | "supports_completion"
  >[];
  error_log: ErrorLogInstance[];
};

export type ProviderTemplatesResponse = {
  provider_templates: SimplifiedProvider<"name">[];
};

export const providersEndpoints = providersApi.endpoints;

export function isProvider(data: unknown): data is Provider {
  if (typeof data !== "object" || data === null) return false;

  if (
    !hasProperty(data, "name") ||
    !hasProperty(data, "endpoint_style") ||
    !hasProperty(data, "chat_endpoint") ||
    !hasProperty(data, "completion_endpoint") ||
    !hasProperty(data, "embedding_endpoint") ||
    !hasProperty(data, "api_key") ||
    !hasProperty(data, "chat_default_model") ||
    !hasProperty(data, "chat_thinking_model") ||
    !hasProperty(data, "chat_light_model") ||
    !hasProperty(data, "enabled")
  )
    return false;

  if (typeof data.name !== "string") return false;
  if (data.endpoint_style !== "openai" && data.endpoint_style !== "hf")
    return false;
  if (typeof data.chat_endpoint !== "string") return false;
  if (typeof data.completion_endpoint !== "string") return false;
  if (typeof data.embedding_endpoint !== "string") return false;
  if (typeof data.api_key !== "string") return false;
  if (typeof data.chat_default_model !== "string") return false;
  if (typeof data.chat_thinking_model !== "string") return false;
  if (typeof data.chat_light_model !== "string") return false;
  if (typeof data.enabled !== "boolean") return false;

  return true;
}

export function isConfiguredProvidersResponse(
  data: unknown,
): data is ConfiguredProvidersResponse {
  // Check if data is an object
  if (typeof data !== "object" || data === null) return false;

  if (!hasProperty(data, "providers") || !hasProperty(data, "error_log"))
    return false;

  if (!Array.isArray(data.providers)) return false;

  if (!Array.isArray(data.error_log)) return false;

  for (const provider of data.providers) {
    if (!isSimplifiedProvider(provider)) return false;
  }

  for (const errorLog of data.error_log) {
    if (!isErrorLogInstance(errorLog)) return false;
  }

  return true;
}

export function isProviderTemplatesResponse(
  data: unknown,
): data is ProviderTemplatesResponse {
  if (typeof data !== "object" || data === null) return false;

  if (!hasProperty(data, "provider_templates")) return false;

  if (!Array.isArray(data.provider_templates)) return false;

  for (const template of data.provider_templates) {
    if (!isSimplifiedProviderWithName(template)) return false;
  }

  return true;
}

function isSimplifiedProviderWithName(
  template: unknown,
): template is SimplifiedProvider<"name"> {
  if (typeof template !== "object" || template === null) return false;

  if (!hasProperty(template, "name")) return false;

  return typeof template.name === "string";
}

function isSimplifiedProvider(
  provider: unknown,
): provider is SimplifiedProvider<"name" | "enabled"> {
  if (typeof provider !== "object" || provider === null) return false;

  if (!hasProperty(provider, "name") || !hasProperty(provider, "enabled"))
    return false;

  if (
    hasProperty(provider, "readonly") &&
    typeof provider.readonly !== "boolean"
  )
    return false;

  return (
    typeof provider.name === "string" && typeof provider.enabled === "boolean"
  );
}

function isErrorLogInstance(errorLog: unknown): errorLog is ErrorLogInstance {
  if (typeof errorLog !== "object" || errorLog === null) return false;

  if (
    !hasProperty(errorLog, "path") ||
    !hasProperty(errorLog, "error_line") ||
    !hasProperty(errorLog, "error_msg")
  )
    return false;

  return (
    typeof errorLog.path === "string" &&
    typeof errorLog.error_line === "number" &&
    typeof errorLog.error_msg === "string"
  );
}
