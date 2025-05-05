import { RootState } from "../../app/store";
import {
  COMPLETION_MODEL_FAMILIES_URL,
  MODEL_DEFAULTS_URL,
  MODEL_URL,
  MODELS_URL,
} from "./consts";
import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { hasProperty } from "../../utils";
import { isDetailMessage } from "./commands";

export const modelsApi = createApi({
  reducerPath: "models",
  tagTypes: ["MODELS", "MODEL"],
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
    getModels: builder.query<ModelsResponse, GetModelsArgs>({
      providesTags: ["MODELS"],
      queryFn: async (args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${MODELS_URL}`;

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
        if (!isModelsResponse(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/models",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
    getModel: builder.query<Model, GetModelArgs>({
      providesTags: ["MODEL"],
      queryFn: async (args, api, extraOptions, baseQuery) => {
        const { modelName, modelType, providerName } = args;

        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${MODEL_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "GET",
          url,
          params: {
            provider: providerName,
            model: modelName,
            type: modelType,
          },
          credentials: "same-origin",
          redirect: "follow",
        });
        if (result.error) {
          return { error: result.error };
        }
        if (!isModel(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/model",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
    getModelDefaults: builder.query<Model, GetModelDefaultsArgs>({
      queryFn: async (args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${MODEL_DEFAULTS_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "GET",
          url,
          params: {
            provider: args.providerName,
            type: args.modelType,
          },
        });

        if (result.error) {
          return { error: result.error };
        }

        if (!isModel(result.data)) {
          return {
            error: {
              error: "Invalid response from /v1/model-defaults",
              status: "CUSTOM_ERROR",
              data: result.data,
            },
          };
        }

        return { data: result.data };
      },
    }),
    getCompletionModelFamilies: builder.query<
      CompletionModelFamiliesResponse,
      undefined
    >({
      queryFn: async (_args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${COMPLETION_MODEL_FAMILIES_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "GET",
          url,
        });

        if (result.error) {
          return { error: result.error };
        }

        if (!isCompletionModelFamiliesResponse(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/completion-model-families",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
    updateModel: builder.mutation<unknown, UpdateModelRequestBody>({
      invalidatesTags: (_result, _error, args) => [
        { type: "MODEL", id: args.model.name },
      ],
      queryFn: async (args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${MODEL_URL}`;

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

        // TODO: this doesn't really work, RTK Query gets FETCH_ERROR is request is failed and is dropping off actual response from the LSP :/
        if (isDetailMessage(result.data)) {
          return {
            meta: result.meta,
            error: {
              error: "Invalid response from /v1/model",
              data: result.data,
              status: "CUSTOM_ERROR",
            },
          };
        }

        return { data: result.data };
      },
    }),
    deleteModel: builder.mutation<unknown, DeleteModelRequestBody>({
      invalidatesTags: (_result, _error, args) => [
        { type: "MODEL", id: args.model },
      ],
      queryFn: async (args, api, extraOptions, baseQuery) => {
        const state = api.getState() as RootState;
        const port = state.config.lspPort as unknown as number;
        const url = `http://127.0.0.1:${port}${MODEL_URL}`;

        const result = await baseQuery({
          ...extraOptions,
          method: "DELETE",
          url,
          params: { ...args },
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
              error: "Invalid response from /v1/model",
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

export type SimplifiedModel = {
  name: string;
  enabled: boolean;
  removable: boolean;
  user_configured: boolean;
};

export type ModelsResponse = {
  completion_models: SimplifiedModel[];
  chat_models: SimplifiedModel[];
  embedding_model: SimplifiedModel;
};

export type ModelType = "embedding" | "completion" | "chat";

export type GetModelArgs = {
  modelName: string;
  providerName: string;
  modelType: ModelType;
};

export type GetModelDefaultsArgs = Omit<GetModelArgs, "modelName">;

export type GetModelsArgs = {
  providerName: string;
};

export type UpdateModelRequestBody = {
  provider: string;
  model: Model;
  type: ModelType;
};

export type DeleteModelRequestBody = Omit<UpdateModelRequestBody, "model"> & {
  model: string;
};

export type SupportsReasoningStyle = "openai" | "anthropic" | "deepseek" | null;

export type CodeChatModel = {
  n_ctx: number;
  name: string;
  tokenizer: string;
  id: string;

  supports_tools: boolean;
  supports_multimodality: boolean;
  supports_clicks: boolean;
  supports_agent: boolean;
  supports_reasoning: SupportsReasoningStyle;
  supports_boost_reasoning: boolean;
  default_temperature: number | null;

  enabled: boolean;

  type: "chat";
};

export type CodeCompletionModel = {
  n_ctx: number;
  name: string;
  model_family: string | null;
  type: "completion";
  enabled: boolean;
};

export type EmbeddingModel = {
  n_ctx: number;
  name: string;
  id: string;
  tokenizer: string;

  embedding_size: number;
  rejection_threshold: number;
  embedding_batch: number;

  enabled: boolean;

  type: "embedding";
};

export function isModelsResponse(data: unknown): data is ModelsResponse {
  // Check if data is an object
  if (typeof data !== "object" || data === null) return false;

  if (
    !hasProperty(data, "completion_models") ||
    !hasProperty(data, "chat_models") ||
    !hasProperty(data, "embedding_model")
  )
    return false;

  return true;
}

export type Model = CodeChatModel | CodeCompletionModel | EmbeddingModel;

export function isCodeChatModel(data: unknown): data is CodeChatModel {
  if (!data || typeof data !== "object") return false;

  if (!("n_ctx" in data) || typeof data.n_ctx !== "number") return false;
  if (!("name" in data) || typeof data.name !== "string") return false;
  if (!("tokenizer" in data) || typeof data.tokenizer !== "string")
    return false;

  if (!("supports_tools" in data) || typeof data.supports_tools !== "boolean")
    return false;
  if (
    !("supports_multimodality" in data) ||
    typeof data.supports_multimodality !== "boolean"
  )
    return false;
  if (!("supports_clicks" in data) || typeof data.supports_clicks !== "boolean")
    return false;
  if (!("supports_agent" in data) || typeof data.supports_agent !== "boolean")
    return false;

  if (!("supports_reasoning" in data)) return false;

  if (
    !("supports_boost_reasoning" in data) ||
    typeof data.supports_boost_reasoning !== "boolean"
  )
    return false;

  if (!("default_temperature" in data)) return false;
  if (
    data.default_temperature !== null &&
    typeof data.default_temperature !== "number"
  )
    return false;

  if (!("enabled" in data) || typeof data.enabled !== "boolean") return false;

  return true;
}

export function isCodeCompletionModel(
  data: unknown,
): data is CodeCompletionModel {
  if (!data || typeof data !== "object") return false;

  if (!("n_ctx" in data) || typeof data.n_ctx !== "number") return false;
  if (!("name" in data) || typeof data.name !== "string") return false;
  if (
    "model_family" in data &&
    typeof data.model_family !== "string" &&
    data.model_family !== null
  )
    return false;
  if (!("enabled" in data) || typeof data.enabled !== "boolean") return false;

  return true;
}

export function isEmbeddingModel(data: unknown): data is EmbeddingModel {
  if (!data || typeof data !== "object") return false;

  if (!("n_ctx" in data) || typeof data.n_ctx !== "number") return false;
  if (!("name" in data) || typeof data.name !== "string") return false;
  if (!("tokenizer" in data) || typeof data.tokenizer !== "string")
    return false;

  if (!("embedding_size" in data) || typeof data.embedding_size !== "number")
    return false;
  if (
    !("rejection_threshold" in data) ||
    typeof data.rejection_threshold !== "number"
  )
    return false;
  if (!("embedding_batch" in data) || typeof data.embedding_batch !== "number")
    return false;

  if (!("enabled" in data) || typeof data.enabled !== "boolean") return false;

  return true;
}

export function isModel(data: unknown): data is Model {
  return (
    isCodeChatModel(data) ||
    isCodeCompletionModel(data) ||
    isEmbeddingModel(data)
  );
}

export type CompletionModelFamiliesResponse = { model_families: string[] };

export function isCompletionModelFamiliesResponse(
  data: unknown,
): data is CompletionModelFamiliesResponse {
  if (!data || typeof data !== "object") return false;
  return "model_families" in data && Array.isArray(data.model_families);
}
