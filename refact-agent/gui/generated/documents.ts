import { z } from "zod";
import type { TypedDocumentNode as DocumentNode } from "@graphql-typed-document-node/core";
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = {
  [K in keyof T]: T[K];
};
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & {
  [SubKey in K]?: Maybe<T[SubKey]>;
};
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & {
  [SubKey in K]: Maybe<T[SubKey]>;
};
export type MakeEmpty<
  T extends { [key: string]: unknown },
  K extends keyof T,
> = { [_ in K]?: never };
export type Incremental<T> =
  | T
  | {
      [P in keyof T]?: P extends " $fragmentName" | "__typename" ? T[P] : never;
    };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string };
  String: { input: string; output: string };
  Boolean: { input: boolean; output: boolean };
  Int: { input: number; output: number };
  Float: { input: number; output: number };
  JSON: { input: any; output: any };
};

export type FExpertInput = {
  fexp_allow_tools: Scalars["String"]["input"];
  fexp_block_tools: Scalars["String"]["input"];
  fexp_name: Scalars["String"]["input"];
  fexp_python_kernel: Scalars["String"]["input"];
  fexp_system_prompt: Scalars["String"]["input"];
  fexp_ver_major: Scalars["Int"]["input"];
  fexp_ver_minor: Scalars["Int"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  owner_fuser_id: Scalars["String"]["input"];
  owner_shared: Scalars["Boolean"]["input"];
};

export type FExpertOutput = {
  __typename?: "FExpertOutput";
  fexp_allow_tools: Scalars["String"]["output"];
  fexp_block_tools: Scalars["String"]["output"];
  fexp_name: Scalars["String"]["output"];
  fexp_python_kernel: Scalars["String"]["output"];
  fexp_system_prompt: Scalars["String"]["output"];
  fexp_ver_major: Scalars["Int"]["output"];
  fexp_ver_minor: Scalars["Int"]["output"];
  located_fgroup_id: Scalars["String"]["output"];
  owner_fuser_id: Scalars["String"]["output"];
  owner_shared: Scalars["Boolean"]["output"];
};

export type FExpertPatch = {
  located_fgroup_id?: InputMaybe<Scalars["String"]["input"]>;
  owner_shared?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type FExpertSubs = {
  __typename?: "FExpertSubs";
  news_action: Scalars["String"]["output"];
  news_payload: FExpertOutput;
  news_payload_id: Scalars["String"]["output"];
};

export type FKnowledgeItem = {
  __typename?: "FKnowledgeItem";
  iknow_created_ts: Scalars["Float"]["output"];
  iknow_goal: Scalars["String"]["output"];
  iknow_id: Scalars["String"]["output"];
  iknow_memory: Scalars["String"]["output"];
  iknow_modified_ts: Scalars["Float"]["output"];
  iknow_origin: Scalars["String"]["output"];
  iknow_stat_correct: Scalars["Int"]["output"];
  iknow_stat_relevant: Scalars["Int"]["output"];
  iknow_stat_times_used: Scalars["Int"]["output"];
  iknow_title: Scalars["String"]["output"];
  iknow_type: Scalars["String"]["output"];
  located_fgroup_id: Scalars["String"]["output"];
  owner_fuser_id: Scalars["String"]["output"];
  owner_shared: Scalars["Boolean"]["output"];
};

export type FKnowledgeItemInput = {
  iknow_goal: Scalars["String"]["input"];
  iknow_memory: Scalars["String"]["input"];
  iknow_title: Scalars["String"]["input"];
  iknow_type: Scalars["String"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  owner_shared: Scalars["Boolean"]["input"];
};

export type FKnowledgeItemPatch = {
  iknow_goal?: InputMaybe<Scalars["String"]["input"]>;
  iknow_memory?: InputMaybe<Scalars["String"]["input"]>;
  iknow_origin?: InputMaybe<Scalars["String"]["input"]>;
  iknow_title?: InputMaybe<Scalars["String"]["input"]>;
  iknow_type?: InputMaybe<Scalars["String"]["input"]>;
  located_fgroup_id?: InputMaybe<Scalars["String"]["input"]>;
  owner_shared?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type FKnowledgeItemSubs = {
  __typename?: "FKnowledgeItemSubs";
  news_action: Scalars["String"]["output"];
  news_payload?: Maybe<FKnowledgeItem>;
  news_payload_id: Scalars["String"]["output"];
};

export type FThread = {
  __typename?: "FThread";
  ft_anything_new: Scalars["Boolean"]["output"];
  ft_archived_ts: Scalars["Float"]["output"];
  ft_belongs_to_fce_id?: Maybe<Scalars["String"]["output"]>;
  ft_created_ts: Scalars["Float"]["output"];
  ft_error: Scalars["String"]["output"];
  ft_fexp_name: Scalars["String"]["output"];
  ft_fexp_ver_major: Scalars["Int"]["output"];
  ft_fexp_ver_minor: Scalars["Int"]["output"];
  ft_id: Scalars["String"]["output"];
  ft_locked_by: Scalars["String"]["output"];
  ft_max_new_tokens: Scalars["Int"]["output"];
  ft_model: Scalars["String"]["output"];
  ft_n: Scalars["Int"]["output"];
  ft_need_assistant: Scalars["Int"]["output"];
  ft_need_tool_calls: Scalars["Int"]["output"];
  ft_temperature: Scalars["Float"]["output"];
  ft_title: Scalars["String"]["output"];
  ft_toolset: Scalars["String"]["output"];
  ft_updated_ts: Scalars["Float"]["output"];
  located_fgroup_id: Scalars["String"]["output"];
  owner_fuser_id: Scalars["String"]["output"];
  owner_shared: Scalars["Boolean"]["output"];
};

export type FThreadDelta = {
  __typename?: "FThreadDelta";
  ftm_content: Scalars["JSON"]["output"];
  ftm_role: Scalars["String"]["output"];
};

export type FThreadInput = {
  ft_belongs_to_fce_id?: InputMaybe<Scalars["String"]["input"]>;
  ft_fexp_name: Scalars["String"]["input"];
  ft_fexp_ver_major: Scalars["Int"]["input"];
  ft_fexp_ver_minor: Scalars["Int"]["input"];
  ft_max_new_tokens?: Scalars["Int"]["input"];
  ft_model?: Scalars["String"]["input"];
  ft_n?: Scalars["Int"]["input"];
  ft_temperature?: Scalars["Float"]["input"];
  ft_title: Scalars["String"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  owner_shared: Scalars["Boolean"]["input"];
};

export type FThreadMessage = {
  __typename?: "FThreadMessage";
  ftm_alt: Scalars["Int"]["output"];
  ftm_belongs_to_ft_id: Scalars["String"]["output"];
  ftm_call_id: Scalars["String"]["output"];
  ftm_content: Scalars["JSON"]["output"];
  ftm_created_ts: Scalars["Float"]["output"];
  ftm_num: Scalars["Int"]["output"];
  ftm_prev_alt: Scalars["Int"]["output"];
  ftm_role: Scalars["String"]["output"];
  ftm_tool_calls?: Maybe<Scalars["JSON"]["output"]>;
  ftm_usage?: Maybe<Scalars["JSON"]["output"]>;
};

export type FThreadMessageInput = {
  ftm_alt: Scalars["Int"]["input"];
  ftm_belongs_to_ft_id: Scalars["String"]["input"];
  ftm_call_id: Scalars["String"]["input"];
  ftm_content: Scalars["String"]["input"];
  ftm_num: Scalars["Int"]["input"];
  ftm_prev_alt: Scalars["Int"]["input"];
  ftm_provenance: Scalars["String"]["input"];
  ftm_role: Scalars["String"]["input"];
  ftm_tool_calls: Scalars["String"]["input"];
  ftm_usage: Scalars["String"]["input"];
};

export type FThreadMessageSubs = {
  __typename?: "FThreadMessageSubs";
  news_action: Scalars["String"]["output"];
  news_payload?: Maybe<FThreadMessage>;
  news_payload_id: Scalars["String"]["output"];
  stream_delta?: Maybe<FThreadDelta>;
};

export type FThreadMessagesCreateResult = {
  __typename?: "FThreadMessagesCreateResult";
  count: Scalars["Int"]["output"];
  messages: Array<FThreadMessage>;
};

export type FThreadMultipleMessagesInput = {
  ftm_belongs_to_ft_id: Scalars["String"]["input"];
  messages: Array<FThreadMessageInput>;
};

export type FThreadPatch = {
  ft_anything_new?: InputMaybe<Scalars["Boolean"]["input"]>;
  ft_archived_ts?: InputMaybe<Scalars["Float"]["input"]>;
  ft_belongs_to_fce_id?: InputMaybe<Scalars["String"]["input"]>;
  ft_error?: InputMaybe<Scalars["String"]["input"]>;
  ft_max_new_tokens?: InputMaybe<Scalars["Int"]["input"]>;
  ft_model?: InputMaybe<Scalars["String"]["input"]>;
  ft_n?: InputMaybe<Scalars["Int"]["input"]>;
  ft_temperature?: InputMaybe<Scalars["Float"]["input"]>;
  ft_title?: InputMaybe<Scalars["String"]["input"]>;
  ft_toolset?: InputMaybe<Scalars["String"]["input"]>;
  located_fgroup_id?: InputMaybe<Scalars["String"]["input"]>;
  owner_shared?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type FThreadSubs = {
  __typename?: "FThreadSubs";
  news_action: Scalars["String"]["output"];
  news_payload?: Maybe<FThread>;
  news_payload_id: Scalars["String"]["output"];
};

export type FlexusGroup = {
  __typename?: "FlexusGroup";
  fgroup_created_ts: Scalars["Float"]["output"];
  fgroup_id: Scalars["String"]["output"];
  fgroup_name: Scalars["String"]["output"];
  fgroup_parent_id?: Maybe<Scalars["String"]["output"]>;
  ws_id: Scalars["String"]["output"];
};

export type FlexusGroupInput = {
  fgroup_name: Scalars["String"]["input"];
  fgroup_parent_id: Scalars["String"]["input"];
};

export type FlexusGroupPatch = {
  fgroup_name?: InputMaybe<Scalars["String"]["input"]>;
  fgroup_parent_id?: InputMaybe<Scalars["String"]["input"]>;
};

export type Mutation = {
  __typename?: "Mutation";
  expert_create: FExpertOutput;
  expert_delete: Scalars["Boolean"]["output"];
  expert_patch: FExpertOutput;
  group_create: FlexusGroup;
  group_delete: Scalars["String"]["output"];
  group_patch: FlexusGroup;
  knowledge_item_create: FKnowledgeItem;
  knowledge_item_delete: Scalars["Boolean"]["output"];
  knowledge_item_patch: FKnowledgeItem;
  thread_create: FThread;
  thread_delete: Scalars["Boolean"]["output"];
  thread_message_create: FThreadMessage;
  thread_messages_create_multiple: FThreadMessagesCreateResult;
  thread_patch: FThread;
};

export type MutationExpert_CreateArgs = {
  input: FExpertInput;
};

export type MutationExpert_DeleteArgs = {
  id: Scalars["String"]["input"];
};

export type MutationExpert_PatchArgs = {
  id: Scalars["String"]["input"];
  patch: FExpertPatch;
};

export type MutationGroup_CreateArgs = {
  input: FlexusGroupInput;
};

export type MutationGroup_DeleteArgs = {
  fgroup_id: Scalars["String"]["input"];
};

export type MutationGroup_PatchArgs = {
  fgroup_id: Scalars["String"]["input"];
  patch: FlexusGroupPatch;
};

export type MutationKnowledge_Item_CreateArgs = {
  input: FKnowledgeItemInput;
};

export type MutationKnowledge_Item_DeleteArgs = {
  id: Scalars["String"]["input"];
};

export type MutationKnowledge_Item_PatchArgs = {
  id: Scalars["String"]["input"];
  patch: FKnowledgeItemPatch;
};

export type MutationThread_CreateArgs = {
  input: FThreadInput;
};

export type MutationThread_DeleteArgs = {
  id: Scalars["String"]["input"];
};

export type MutationThread_Message_CreateArgs = {
  input: FThreadMessageInput;
};

export type MutationThread_Messages_Create_MultipleArgs = {
  input: FThreadMultipleMessagesInput;
};

export type MutationThread_PatchArgs = {
  id: Scalars["String"]["input"];
  patch: FThreadPatch;
};

export type Query = {
  __typename?: "Query";
  expert_list: Array<FExpertOutput>;
  knowledge_item_list: Array<FKnowledgeItem>;
  thread_list: Array<FThread>;
  thread_messages: Array<FThreadMessage>;
  thread_messages_by_alt: Array<FThreadMessage>;
};

export type QueryExpert_ListArgs = {
  limit: Scalars["Int"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  skip: Scalars["Int"]["input"];
  sort_by?: Scalars["String"]["input"];
};

export type QueryKnowledge_Item_ListArgs = {
  limit: Scalars["Int"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  skip: Scalars["Int"]["input"];
  sort_by?: Scalars["String"]["input"];
};

export type QueryThread_ListArgs = {
  limit: Scalars["Int"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  skip: Scalars["Int"]["input"];
  sort_by?: Scalars["String"]["input"];
};

export type QueryThread_MessagesArgs = {
  alt?: InputMaybe<Scalars["Int"]["input"]>;
  ft_id: Scalars["String"]["input"];
  limit?: InputMaybe<Scalars["Int"]["input"]>;
  num?: InputMaybe<Scalars["Int"]["input"]>;
  role?: InputMaybe<Scalars["String"]["input"]>;
};

export type QueryThread_Messages_By_AltArgs = {
  alt: Scalars["Int"]["input"];
  ft_id: Scalars["String"]["input"];
};

export type Subscription = {
  __typename?: "Subscription";
  comprehensive_thread_subs: FThreadMessageSubs;
  experts_in_group: FExpertSubs;
  knowledge_items_in_group: FKnowledgeItemSubs;
  threads_in_group: FThreadSubs;
  tree_subscription: TreeUpdateSubs;
};

export type SubscriptionComprehensive_Thread_SubsArgs = {
  ft_id: Scalars["String"]["input"];
  want_deltas: Scalars["Boolean"]["input"];
};

export type SubscriptionExperts_In_GroupArgs = {
  limit: Scalars["Int"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  sort_by?: Scalars["String"]["input"];
};

export type SubscriptionKnowledge_Items_In_GroupArgs = {
  limit: Scalars["Int"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  sort_by?: Scalars["String"]["input"];
};

export type SubscriptionThreads_In_GroupArgs = {
  limit: Scalars["Int"]["input"];
  located_fgroup_id: Scalars["String"]["input"];
  sort_by?: Scalars["String"]["input"];
};

export type SubscriptionTree_SubscriptionArgs = {
  ws_id: Scalars["String"]["input"];
};

export type TreeUpdateSubs = {
  __typename?: "TreeUpdateSubs";
  treeupd_action: Scalars["String"]["output"];
  treeupd_id: Scalars["String"]["output"];
  treeupd_path: Scalars["String"]["output"];
  treeupd_title: Scalars["String"]["output"];
  treeupd_type: Scalars["String"]["output"];
};

export type NavTreeSubsSubscriptionVariables = Exact<{
  ws_id: Scalars["String"]["input"];
}>;

export type NavTreeSubsSubscription = {
  __typename?: "Subscription";
  tree_subscription: {
    __typename?: "TreeUpdateSubs";
    treeupd_action: string;
    treeupd_id: string;
    treeupd_path: string;
    treeupd_type: string;
    treeupd_title: string;
  };
};

export const NavTreeSubsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "NavTreeSubs" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "ws_id" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "tree_subscription" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "ws_id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "ws_id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "treeupd_action" },
                },
                { kind: "Field", name: { kind: "Name", value: "treeupd_id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "treeupd_path" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "treeupd_type" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "treeupd_title" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  NavTreeSubsSubscription,
  NavTreeSubsSubscriptionVariables
>;

type Properties<T> = Required<{
  [K in keyof T]: z.ZodType<T[K], any, T[K]>;
}>;

type definedNonNullAny = {};

export const isDefinedNonNullAny = (v: any): v is definedNonNullAny =>
  v !== undefined && v !== null;

export const definedNonNullAnySchema = z
  .any()
  .refine((v) => isDefinedNonNullAny(v));

export function FExpertInputSchema(): z.ZodObject<Properties<FExpertInput>> {
  return z.object({
    fexp_allow_tools: z.string(),
    fexp_block_tools: z.string(),
    fexp_name: z.string(),
    fexp_python_kernel: z.string(),
    fexp_system_prompt: z.string(),
    fexp_ver_major: z.number(),
    fexp_ver_minor: z.number(),
    located_fgroup_id: z.string(),
    owner_fuser_id: z.string(),
    owner_shared: z.boolean(),
  });
}

export function FExpertPatchSchema(): z.ZodObject<Properties<FExpertPatch>> {
  return z.object({
    located_fgroup_id: z.string().nullish(),
    owner_shared: z.boolean().nullish(),
  });
}

export function FKnowledgeItemInputSchema(): z.ZodObject<
  Properties<FKnowledgeItemInput>
> {
  return z.object({
    iknow_goal: z.string(),
    iknow_memory: z.string(),
    iknow_title: z.string(),
    iknow_type: z.string(),
    located_fgroup_id: z.string(),
    owner_shared: z.boolean(),
  });
}

export function FKnowledgeItemPatchSchema(): z.ZodObject<
  Properties<FKnowledgeItemPatch>
> {
  return z.object({
    iknow_goal: z.string().nullish(),
    iknow_memory: z.string().nullish(),
    iknow_origin: z.string().nullish(),
    iknow_title: z.string().nullish(),
    iknow_type: z.string().nullish(),
    located_fgroup_id: z.string().nullish(),
    owner_shared: z.boolean().nullish(),
  });
}

export function FThreadInputSchema(): z.ZodObject<Properties<FThreadInput>> {
  return z.object({
    ft_belongs_to_fce_id: z.string().nullish(),
    ft_fexp_name: z.string(),
    ft_fexp_ver_major: z.number(),
    ft_fexp_ver_minor: z.number(),
    ft_max_new_tokens: z.number().default(8192),
    ft_model: z.string().default(""),
    ft_n: z.number().default(1),
    ft_temperature: z.number().default(0),
    ft_title: z.string(),
    located_fgroup_id: z.string(),
    owner_shared: z.boolean(),
  });
}

export function FThreadMessageInputSchema(): z.ZodObject<
  Properties<FThreadMessageInput>
> {
  return z.object({
    ftm_alt: z.number(),
    ftm_belongs_to_ft_id: z.string(),
    ftm_call_id: z.string(),
    ftm_content: z.string(),
    ftm_num: z.number(),
    ftm_prev_alt: z.number(),
    ftm_provenance: z.string(),
    ftm_role: z.string(),
    ftm_tool_calls: z.string(),
    ftm_usage: z.string(),
  });
}

export function FThreadMultipleMessagesInputSchema(): z.ZodObject<
  Properties<FThreadMultipleMessagesInput>
> {
  return z.object({
    ftm_belongs_to_ft_id: z.string(),
    messages: z.array(z.lazy(() => FThreadMessageInputSchema())),
  });
}

export function FThreadPatchSchema(): z.ZodObject<Properties<FThreadPatch>> {
  return z.object({
    ft_anything_new: z.boolean().nullish(),
    ft_archived_ts: z.number().nullish(),
    ft_belongs_to_fce_id: z.string().nullish(),
    ft_error: z.string().nullish(),
    ft_max_new_tokens: z.number().nullish(),
    ft_model: z.string().nullish(),
    ft_n: z.number().nullish(),
    ft_temperature: z.number().nullish(),
    ft_title: z.string().nullish(),
    ft_toolset: z.string().nullish(),
    located_fgroup_id: z.string().nullish(),
    owner_shared: z.boolean().nullish(),
  });
}

export function FlexusGroupInputSchema(): z.ZodObject<
  Properties<FlexusGroupInput>
> {
  return z.object({
    fgroup_name: z.string(),
    fgroup_parent_id: z.string(),
  });
}

export function FlexusGroupPatchSchema(): z.ZodObject<
  Properties<FlexusGroupPatch>
> {
  return z.object({
    fgroup_name: z.string().nullish(),
    fgroup_parent_id: z.string().nullish(),
  });
}
