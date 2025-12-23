import { createApi, fetchBaseQuery } from "@reduxjs/toolkit/query/react";
import { ChatThread } from "../../features/Chat/Thread/types";
import { ChatMessages } from "./types";

export type TrajectoryMeta = {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  model: string;
  mode: string;
  message_count: number;
};

export type TrajectoryData = {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  model: string;
  mode: string;
  tool_use: string;
  messages: ChatMessages;
  boost_reasoning?: boolean;
  context_tokens_cap?: number;
  include_project_info?: boolean;
  increase_max_tokens?: boolean;
  automatic_patch?: boolean;
  project_name?: string;
  read?: boolean;
  isTitleGenerated?: boolean;
};

export type TrajectoryEvent = {
  type: "created" | "updated" | "deleted";
  id: string;
  updated_at?: string;
  title?: string;
};

export function chatThreadToTrajectoryData(thread: ChatThread, createdAt?: string): TrajectoryData {
  const now = new Date().toISOString();
  return {
    id: thread.id,
    title: thread.title || "New Chat",
    created_at: createdAt || now,
    updated_at: now,
    model: thread.model,
    mode: thread.mode || "AGENT",
    tool_use: thread.tool_use || "agent",
    messages: thread.messages,
    boost_reasoning: thread.boost_reasoning,
    context_tokens_cap: thread.context_tokens_cap,
    include_project_info: thread.include_project_info,
    increase_max_tokens: thread.increase_max_tokens,
    automatic_patch: thread.automatic_patch,
    project_name: thread.project_name,
    read: thread.read,
    isTitleGenerated: thread.isTitleGenerated,
  };
}

export function trajectoryDataToChatThread(data: TrajectoryData): ChatThread {
  return {
    id: data.id,
    title: data.title,
    model: data.model,
    mode: data.mode as ChatThread["mode"],
    tool_use: data.tool_use as ChatThread["tool_use"],
    messages: data.messages,
    boost_reasoning: data.boost_reasoning ?? false,
    context_tokens_cap: data.context_tokens_cap,
    include_project_info: data.include_project_info ?? true,
    increase_max_tokens: data.increase_max_tokens ?? false,
    automatic_patch: data.automatic_patch ?? false,
    project_name: data.project_name,
    read: data.read,
    isTitleGenerated: data.isTitleGenerated,
    createdAt: data.created_at,
    last_user_message_id: "",
    new_chat_suggested: { wasSuggested: false },
  };
}

export const trajectoriesApi = createApi({
  reducerPath: "trajectoriesApi",
  baseQuery: fetchBaseQuery({ baseUrl: "/v1" }),
  tagTypes: ["Trajectory"],
  endpoints: (builder) => ({
    listTrajectories: builder.query<TrajectoryMeta[], void>({
      query: () => "/trajectories",
      providesTags: ["Trajectory"],
    }),
    getTrajectory: builder.query<TrajectoryData, string>({
      query: (id) => `/trajectories/${id}`,
      providesTags: (_result, _error, id) => [{ type: "Trajectory", id }],
    }),
    saveTrajectory: builder.mutation<void, TrajectoryData>({
      query: (data) => ({
        url: `/trajectories/${data.id}`,
        method: "PUT",
        body: data,
      }),
      invalidatesTags: (_result, _error, data) => [
        { type: "Trajectory", id: data.id },
        "Trajectory",
      ],
    }),
    deleteTrajectory: builder.mutation<void, string>({
      query: (id) => ({
        url: `/trajectories/${id}`,
        method: "DELETE",
      }),
      invalidatesTags: ["Trajectory"],
    }),
  }),
});

export const {
  useListTrajectoriesQuery,
  useGetTrajectoryQuery,
  useSaveTrajectoryMutation,
  useDeleteTrajectoryMutation,
} = trajectoriesApi;
