import { z } from 'zod'
import type { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string; }
  String: { input: string; output: string; }
  Boolean: { input: boolean; output: boolean; }
  Int: { input: number; output: number; }
  Float: { input: number; output: number; }
  JSON: { input: any; output: any; }
};

export type BasicStuffResult = {
  __typename?: 'BasicStuffResult';
  fuser_id: Scalars['String']['output'];
  workspaces: Array<FWorkspace>;
};

export type FExpertInput = {
  fexp_allow_tools: Scalars['String']['input'];
  fexp_block_tools: Scalars['String']['input'];
  fexp_name: Scalars['String']['input'];
  fexp_python_kernel: Scalars['String']['input'];
  fexp_system_prompt: Scalars['String']['input'];
  located_fgroup_id: Scalars['String']['input'];
  owner_fuser_id?: InputMaybe<Scalars['String']['input']>;
  owner_shared: Scalars['Boolean']['input'];
};

export type FExpertOutput = {
  __typename?: 'FExpertOutput';
  fexp_allow_tools: Scalars['String']['output'];
  fexp_block_tools: Scalars['String']['output'];
  fexp_name: Scalars['String']['output'];
  fexp_python_kernel: Scalars['String']['output'];
  fexp_system_prompt: Scalars['String']['output'];
  located_fgroup_id?: Maybe<Scalars['String']['output']>;
  owner_fuser_id?: Maybe<Scalars['String']['output']>;
  owner_shared: Scalars['Boolean']['output'];
};

export type FExpertPatch = {
  located_fgroup_id?: InputMaybe<Scalars['String']['input']>;
  owner_shared?: InputMaybe<Scalars['Boolean']['input']>;
};

export type FExpertSubs = {
  __typename?: 'FExpertSubs';
  news_action: Scalars['String']['output'];
  news_payload: FExpertOutput;
  news_payload_id: Scalars['String']['output'];
  news_pubsub: Scalars['String']['output'];
};

export type FExternalDataSourceInput = {
  eds_json: Scalars['String']['input'];
  eds_name: Scalars['String']['input'];
  eds_type: Scalars['String']['input'];
  located_fgroup_id: Scalars['String']['input'];
};

export type FExternalDataSourceOutput = {
  __typename?: 'FExternalDataSourceOutput';
  eds_created_ts: Scalars['Float']['output'];
  eds_id: Scalars['String']['output'];
  eds_json: Scalars['JSON']['output'];
  eds_last_successful_scan_ts: Scalars['Float']['output'];
  eds_modified_ts: Scalars['Float']['output'];
  eds_name: Scalars['String']['output'];
  eds_scan_status: Scalars['String']['output'];
  eds_secret_id?: Maybe<Scalars['Int']['output']>;
  eds_type: Scalars['String']['output'];
  located_fgroup_id: Scalars['String']['output'];
  owner_fuser_id: Scalars['String']['output'];
};

export type FExternalDataSourcePatch = {
  eds_json: Scalars['String']['input'];
  eds_last_successful_scan_ts?: InputMaybe<Scalars['Float']['input']>;
  eds_name?: InputMaybe<Scalars['String']['input']>;
  eds_scan_status?: InputMaybe<Scalars['String']['input']>;
  eds_secret_id?: InputMaybe<Scalars['Int']['input']>;
  eds_type?: InputMaybe<Scalars['String']['input']>;
  located_fgroup_id?: InputMaybe<Scalars['String']['input']>;
};

export type FExternalDataSourceSubs = {
  __typename?: 'FExternalDataSourceSubs';
  news_action: Scalars['String']['output'];
  news_payload?: Maybe<FExternalDataSourceOutput>;
  news_payload_id: Scalars['String']['output'];
};

export type FKnowledgeItemInput = {
  iknow_is_core?: Scalars['Boolean']['input'];
  iknow_memory: Scalars['String']['input'];
  iknow_tags?: Array<Scalars['String']['input']>;
  located_fgroup_id: Scalars['String']['input'];
  owner_shared: Scalars['Boolean']['input'];
};

export type FKnowledgeItemOutput = {
  __typename?: 'FKnowledgeItemOutput';
  iknow_created_ts: Scalars['Float']['output'];
  iknow_id: Scalars['String']['output'];
  iknow_is_core: Scalars['Boolean']['output'];
  iknow_memory: Scalars['String']['output'];
  iknow_modified_ts: Scalars['Float']['output'];
  iknow_stat_correct: Scalars['Int']['output'];
  iknow_stat_relevant: Scalars['Int']['output'];
  iknow_stat_times_used: Scalars['Int']['output'];
  iknow_tags: Array<Scalars['String']['output']>;
  located_fgroup_id: Scalars['String']['output'];
  owner_fuser_id: Scalars['String']['output'];
  owner_shared: Scalars['Boolean']['output'];
};

export type FKnowledgeItemPatch = {
  iknow_is_core?: InputMaybe<Scalars['Boolean']['input']>;
  iknow_memory?: InputMaybe<Scalars['String']['input']>;
  iknow_tags?: InputMaybe<Array<Scalars['String']['input']>>;
  located_fgroup_id?: InputMaybe<Scalars['String']['input']>;
  owner_shared?: InputMaybe<Scalars['Boolean']['input']>;
};

export type FKnowledgeItemSubs = {
  __typename?: 'FKnowledgeItemSubs';
  news_action: Scalars['String']['output'];
  news_payload?: Maybe<FKnowledgeItemOutput>;
  news_payload_id: Scalars['String']['output'];
  news_pubsub: Scalars['String']['output'];
};

export type FPluginOutput = {
  __typename?: 'FPluginOutput';
  plugin_name: Scalars['String']['output'];
  plugin_setup_page: Scalars['String']['output'];
  plugin_version: Scalars['String']['output'];
};

export type FThreadDelta = {
  __typename?: 'FThreadDelta';
  ftm_content: Scalars['JSON']['output'];
  ftm_role: Scalars['String']['output'];
};

export type FThreadInput = {
  ft_app_capture?: Scalars['String']['input'];
  ft_app_searchable?: Scalars['String']['input'];
  ft_app_specific?: Scalars['String']['input'];
  ft_belongs_to_fce_id?: InputMaybe<Scalars['String']['input']>;
  ft_error?: Scalars['String']['input'];
  ft_fexp_name: Scalars['String']['input'];
  ft_max_new_tokens?: Scalars['Int']['input'];
  ft_model?: Scalars['String']['input'];
  ft_n?: Scalars['Int']['input'];
  ft_temperature?: Scalars['Float']['input'];
  ft_title: Scalars['String']['input'];
  ft_toolset?: Scalars['String']['input'];
  located_fgroup_id: Scalars['String']['input'];
  owner_shared: Scalars['Boolean']['input'];
};

export type FThreadMessageInput = {
  ftm_alt: Scalars['Int']['input'];
  ftm_app_specific?: InputMaybe<Scalars['String']['input']>;
  ftm_belongs_to_ft_id: Scalars['String']['input'];
  ftm_call_id: Scalars['String']['input'];
  ftm_content?: InputMaybe<Scalars['String']['input']>;
  ftm_num: Scalars['Int']['input'];
  ftm_prev_alt: Scalars['Int']['input'];
  ftm_provenance: Scalars['String']['input'];
  ftm_role: Scalars['String']['input'];
  ftm_tool_calls?: InputMaybe<Scalars['String']['input']>;
  ftm_usage?: InputMaybe<Scalars['String']['input']>;
};

export type FThreadMessageOutput = {
  __typename?: 'FThreadMessageOutput';
  ft_app_capture?: Maybe<Scalars['String']['output']>;
  ft_app_searchable?: Maybe<Scalars['String']['output']>;
  ft_app_specific?: Maybe<Scalars['JSON']['output']>;
  ftm_alt: Scalars['Int']['output'];
  ftm_app_specific?: Maybe<Scalars['JSON']['output']>;
  ftm_belongs_to_ft_id: Scalars['String']['output'];
  ftm_call_id: Scalars['String']['output'];
  ftm_content?: Maybe<Scalars['JSON']['output']>;
  ftm_created_ts: Scalars['Float']['output'];
  ftm_num: Scalars['Int']['output'];
  ftm_prev_alt: Scalars['Int']['output'];
  ftm_provenance: Scalars['JSON']['output'];
  ftm_role: Scalars['String']['output'];
  ftm_tool_calls?: Maybe<Scalars['JSON']['output']>;
  ftm_usage?: Maybe<Scalars['JSON']['output']>;
};

export type FThreadMessageSubs = {
  __typename?: 'FThreadMessageSubs';
  news_action: Scalars['String']['output'];
  news_payload_id: Scalars['String']['output'];
  news_payload_thread?: Maybe<FThreadOutput>;
  news_payload_thread_message?: Maybe<FThreadMessageOutput>;
  stream_delta?: Maybe<FThreadDelta>;
};

export type FThreadMessagesCreateResult = {
  __typename?: 'FThreadMessagesCreateResult';
  count: Scalars['Int']['output'];
  messages: Array<FThreadMessageOutput>;
};

export type FThreadMultipleMessagesInput = {
  ftm_belongs_to_ft_id: Scalars['String']['input'];
  messages: Array<FThreadMessageInput>;
};

export type FThreadOutput = {
  __typename?: 'FThreadOutput';
  ft_anything_new: Scalars['Boolean']['output'];
  ft_app_capture: Scalars['String']['output'];
  ft_app_searchable: Scalars['String']['output'];
  ft_app_specific?: Maybe<Scalars['JSON']['output']>;
  ft_archived_ts: Scalars['Float']['output'];
  ft_belongs_to_fce_id?: Maybe<Scalars['String']['output']>;
  ft_created_ts: Scalars['Float']['output'];
  ft_error?: Maybe<Scalars['JSON']['output']>;
  ft_fexp_name: Scalars['String']['output'];
  ft_id: Scalars['String']['output'];
  ft_locked_by: Scalars['String']['output'];
  ft_max_new_tokens: Scalars['Int']['output'];
  ft_model: Scalars['String']['output'];
  ft_n: Scalars['Int']['output'];
  ft_need_assistant: Scalars['Int']['output'];
  ft_need_tool_calls: Scalars['Int']['output'];
  ft_temperature: Scalars['Float']['output'];
  ft_title: Scalars['String']['output'];
  ft_toolset?: Maybe<Scalars['JSON']['output']>;
  ft_updated_ts: Scalars['Float']['output'];
  located_fgroup_id: Scalars['String']['output'];
  owner_fuser_id: Scalars['String']['output'];
  owner_shared: Scalars['Boolean']['output'];
};

export type FThreadPatch = {
  ft_anything_new?: InputMaybe<Scalars['Boolean']['input']>;
  ft_app_searchable?: InputMaybe<Scalars['String']['input']>;
  ft_app_specific?: InputMaybe<Scalars['String']['input']>;
  ft_archived_ts?: InputMaybe<Scalars['Float']['input']>;
  ft_belongs_to_fce_id?: InputMaybe<Scalars['String']['input']>;
  ft_error?: InputMaybe<Scalars['String']['input']>;
  ft_max_new_tokens?: InputMaybe<Scalars['Int']['input']>;
  ft_model?: InputMaybe<Scalars['String']['input']>;
  ft_n?: InputMaybe<Scalars['Int']['input']>;
  ft_temperature?: InputMaybe<Scalars['Float']['input']>;
  ft_title?: InputMaybe<Scalars['String']['input']>;
  ft_toolset?: InputMaybe<Scalars['String']['input']>;
  located_fgroup_id?: InputMaybe<Scalars['String']['input']>;
  owner_shared?: InputMaybe<Scalars['Boolean']['input']>;
};

export type FThreadSubs = {
  __typename?: 'FThreadSubs';
  news_action: Scalars['String']['output'];
  news_payload?: Maybe<FThreadOutput>;
  news_payload_id: Scalars['String']['output'];
  news_pubsub: Scalars['String']['output'];
};

export type FWorkspace = {
  __typename?: 'FWorkspace';
  root_group_name: Scalars['String']['output'];
  ws_created_ts: Scalars['Float']['output'];
  ws_id: Scalars['String']['output'];
  ws_owner_fuser_id: Scalars['String']['output'];
  ws_root_group_id: Scalars['String']['output'];
  ws_status: Scalars['String']['output'];
};

export type FlexusGroup = {
  __typename?: 'FlexusGroup';
  fgroup_created_ts: Scalars['Float']['output'];
  fgroup_id: Scalars['String']['output'];
  fgroup_name: Scalars['String']['output'];
  fgroup_parent_id?: Maybe<Scalars['String']['output']>;
  ws_id: Scalars['String']['output'];
};

export type FlexusGroupInput = {
  fgroup_name: Scalars['String']['input'];
  fgroup_parent_id: Scalars['String']['input'];
};

export type FlexusGroupPatch = {
  fgroup_name?: InputMaybe<Scalars['String']['input']>;
  fgroup_parent_id?: InputMaybe<Scalars['String']['input']>;
};

export type Mutation = {
  __typename?: 'Mutation';
  expert_create: FExpertOutput;
  expert_delete: Scalars['Boolean']['output'];
  expert_patch: FExpertOutput;
  external_data_source_create: FExternalDataSourceOutput;
  external_data_source_delete: Scalars['Boolean']['output'];
  external_data_source_patch: FExternalDataSourceOutput;
  group_create: FlexusGroup;
  group_delete: Scalars['String']['output'];
  group_patch: FlexusGroup;
  knowledge_item_create: FKnowledgeItemOutput;
  knowledge_item_delete: Scalars['Boolean']['output'];
  knowledge_item_mass_group_patch: Scalars['Int']['output'];
  knowledge_item_patch: FKnowledgeItemOutput;
  stats_add: Scalars['Boolean']['output'];
  tech_support_activate: Scalars['Boolean']['output'];
  tech_support_set_config: Scalars['Boolean']['output'];
  thread_create: FThreadOutput;
  thread_delete: Scalars['Boolean']['output'];
  thread_lock: Scalars['Boolean']['output'];
  thread_mass_group_patch: Scalars['Int']['output'];
  thread_message_create: FThreadMessageOutput;
  thread_messages_create_multiple: FThreadMessagesCreateResult;
  thread_patch: FThreadOutput;
  thread_provide_toolset: Scalars['Boolean']['output'];
  thread_unlock: Scalars['Boolean']['output'];
};


export type MutationExpert_CreateArgs = {
  input: FExpertInput;
};


export type MutationExpert_DeleteArgs = {
  id: Scalars['String']['input'];
};


export type MutationExpert_PatchArgs = {
  id: Scalars['String']['input'];
  patch: FExpertPatch;
};


export type MutationExternal_Data_Source_CreateArgs = {
  input: FExternalDataSourceInput;
};


export type MutationExternal_Data_Source_DeleteArgs = {
  id: Scalars['String']['input'];
};


export type MutationExternal_Data_Source_PatchArgs = {
  id: Scalars['String']['input'];
  patch: FExternalDataSourcePatch;
};


export type MutationGroup_CreateArgs = {
  input: FlexusGroupInput;
};


export type MutationGroup_DeleteArgs = {
  fgroup_id: Scalars['String']['input'];
};


export type MutationGroup_PatchArgs = {
  fgroup_id: Scalars['String']['input'];
  patch: FlexusGroupPatch;
};


export type MutationKnowledge_Item_CreateArgs = {
  input: FKnowledgeItemInput;
};


export type MutationKnowledge_Item_DeleteArgs = {
  id: Scalars['String']['input'];
};


export type MutationKnowledge_Item_Mass_Group_PatchArgs = {
  dst_group_id: Scalars['String']['input'];
  src_group_id: Scalars['String']['input'];
};


export type MutationKnowledge_Item_PatchArgs = {
  id: Scalars['String']['input'];
  patch: FKnowledgeItemPatch;
};


export type MutationStats_AddArgs = {
  st_how_many: Scalars['Int']['input'];
  st_involved_expert?: Scalars['String']['input'];
  st_involved_fuser_id?: Scalars['String']['input'];
  st_involved_model?: Scalars['String']['input'];
  st_thing: Scalars['String']['input'];
  ts: Scalars['Float']['input'];
  ws_id: Scalars['String']['input'];
};


export type MutationTech_Support_ActivateArgs = {
  ws_id: Scalars['String']['input'];
};


export type MutationTech_Support_Set_ConfigArgs = {
  config: TechSupportSettingsInput;
  ws_id: Scalars['String']['input'];
};


export type MutationThread_CreateArgs = {
  input: FThreadInput;
};


export type MutationThread_DeleteArgs = {
  id: Scalars['String']['input'];
};


export type MutationThread_LockArgs = {
  ft_id: Scalars['String']['input'];
  worker_name: Scalars['String']['input'];
};


export type MutationThread_Mass_Group_PatchArgs = {
  dst_group_id: Scalars['String']['input'];
  src_group_id: Scalars['String']['input'];
};


export type MutationThread_Message_CreateArgs = {
  input: FThreadMessageInput;
};


export type MutationThread_Messages_Create_MultipleArgs = {
  input: FThreadMultipleMessagesInput;
};


export type MutationThread_PatchArgs = {
  id: Scalars['String']['input'];
  patch: FThreadPatch;
};


export type MutationThread_Provide_ToolsetArgs = {
  ft_id: Scalars['String']['input'];
  toolset: Scalars['String']['input'];
};


export type MutationThread_UnlockArgs = {
  ft_id: Scalars['String']['input'];
  worker_name: Scalars['String']['input'];
};

export type Query = {
  __typename?: 'Query';
  expert_get: FExpertOutput;
  expert_list: Array<FExpertOutput>;
  experts_effective_list: Array<FExpertOutput>;
  external_data_source_get: FExternalDataSourceOutput;
  external_data_source_list: Array<FExternalDataSourceOutput>;
  knowledge_item_get: FKnowledgeItemOutput;
  knowledge_item_list: Array<FKnowledgeItemOutput>;
  plugins_installed: Array<FPluginOutput>;
  query_basic_stuff: BasicStuffResult;
  tech_support_get_config?: Maybe<TechSupportSettingsOutput>;
  thread_get: FThreadOutput;
  thread_list: Array<FThreadOutput>;
  thread_messages_list: Array<FThreadMessageOutput>;
  threads_app_captured: Array<FThreadOutput>;
};


export type QueryExpert_GetArgs = {
  id: Scalars['String']['input'];
};


export type QueryExpert_ListArgs = {
  limit: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  skip: Scalars['Int']['input'];
  sort_by?: Scalars['String']['input'];
};


export type QueryExperts_Effective_ListArgs = {
  located_fgroup_id: Scalars['String']['input'];
};


export type QueryExternal_Data_Source_GetArgs = {
  id: Scalars['String']['input'];
};


export type QueryExternal_Data_Source_ListArgs = {
  limit: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  skip: Scalars['Int']['input'];
  sort_by?: Scalars['String']['input'];
};


export type QueryKnowledge_Item_GetArgs = {
  id: Scalars['String']['input'];
};


export type QueryKnowledge_Item_ListArgs = {
  limit: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  skip: Scalars['Int']['input'];
  sort_by?: Scalars['String']['input'];
};


export type QueryTech_Support_Get_ConfigArgs = {
  ws_id: Scalars['String']['input'];
};


export type QueryThread_GetArgs = {
  id: Scalars['String']['input'];
};


export type QueryThread_ListArgs = {
  limit: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  skip: Scalars['Int']['input'];
  sort_by?: Scalars['String']['input'];
};


export type QueryThread_Messages_ListArgs = {
  ft_id: Scalars['String']['input'];
  ftm_alt?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryThreads_App_CapturedArgs = {
  ft_app_capture: Scalars['String']['input'];
  ft_app_searchable: Scalars['String']['input'];
  located_fgroup_id: Scalars['String']['input'];
};

export type Subscription = {
  __typename?: 'Subscription';
  comprehensive_thread_subs: FThreadMessageSubs;
  experts_in_group: FExpertSubs;
  external_data_sources_in_group: FExternalDataSourceSubs;
  knowledge_items_in_group: FKnowledgeItemSubs;
  threads_in_group: FThreadSubs;
  tree_subscription: TreeUpdateSubs;
};


export type SubscriptionComprehensive_Thread_SubsArgs = {
  ft_id: Scalars['String']['input'];
  want_deltas: Scalars['Boolean']['input'];
};


export type SubscriptionExperts_In_GroupArgs = {
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Scalars['String']['input'];
};


export type SubscriptionExternal_Data_Sources_In_GroupArgs = {
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Scalars['String']['input'];
};


export type SubscriptionKnowledge_Items_In_GroupArgs = {
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Scalars['String']['input'];
};


export type SubscriptionThreads_In_GroupArgs = {
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Scalars['String']['input'];
};


export type SubscriptionTree_SubscriptionArgs = {
  ws_id: Scalars['String']['input'];
};

export type TechSupportSettingsInput = {
  support_api_key: Scalars['String']['input'];
  support_channel_list: Array<Scalars['String']['input']>;
  support_discord_key: Scalars['String']['input'];
  support_fgroup_id: Scalars['String']['input'];
  support_fuser_id: Scalars['String']['input'];
};

export type TechSupportSettingsOutput = {
  __typename?: 'TechSupportSettingsOutput';
  support_api_key: Scalars['String']['output'];
  support_channel_list: Array<Scalars['String']['output']>;
  support_discord_key: Scalars['String']['output'];
  support_fgroup_id: Scalars['String']['output'];
  support_fuser_id: Scalars['String']['output'];
};

export type TreeUpdateSubs = {
  __typename?: 'TreeUpdateSubs';
  treeupd_action: Scalars['String']['output'];
  treeupd_id: Scalars['String']['output'];
  treeupd_path: Scalars['String']['output'];
  treeupd_title: Scalars['String']['output'];
  treeupd_type: Scalars['String']['output'];
};

export type CreateGroupMutationVariables = Exact<{
  fgroup_name: Scalars['String']['input'];
  fgroup_parent_id: Scalars['String']['input'];
}>;


export type CreateGroupMutation = { __typename?: 'Mutation', group_create: { __typename?: 'FlexusGroup', fgroup_id: string, fgroup_name: string, ws_id: string, fgroup_parent_id?: string | null, fgroup_created_ts: number } };

export type NavTreeSubsSubscriptionVariables = Exact<{
  ws_id: Scalars['String']['input'];
}>;


export type NavTreeSubsSubscription = { __typename?: 'Subscription', tree_subscription: { __typename?: 'TreeUpdateSubs', treeupd_action: string, treeupd_id: string, treeupd_path: string, treeupd_type: string, treeupd_title: string } };

export type NavTreeWantWorkspacesQueryVariables = Exact<{ [key: string]: never; }>;


export type NavTreeWantWorkspacesQuery = { __typename?: 'Query', query_basic_stuff: { __typename?: 'BasicStuffResult', fuser_id: string, workspaces: Array<{ __typename?: 'FWorkspace', ws_id: string, ws_owner_fuser_id: string, ws_root_group_id: string, root_group_name: string }> } };

export type ThreadsPageSubsSubscriptionVariables = Exact<{
  located_fgroup_id: Scalars['String']['input'];
  limit: Scalars['Int']['input'];
}>;


export type ThreadsPageSubsSubscription = { __typename?: 'Subscription', threads_in_group: { __typename?: 'FThreadSubs', news_action: string, news_payload_id: string, news_payload?: { __typename?: 'FThreadOutput', owner_fuser_id: string, owner_shared: boolean, ft_id: string, ft_title: string, ft_error?: any | null, ft_updated_ts: number, ft_locked_by: string, ft_need_assistant: number, ft_need_tool_calls: number, ft_anything_new: boolean, ft_archived_ts: number, ft_created_ts: number, ft_n: number } | null } };

export type DeleteThreadMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteThreadMutation = { __typename?: 'Mutation', thread_delete: boolean };

export type CreateThreadMutationVariables = Exact<{
  input: FThreadInput;
}>;


export type CreateThreadMutation = { __typename?: 'Mutation', thread_create: { __typename?: 'FThreadOutput', ft_id: string } };

export type MessagesSubscriptionSubscriptionVariables = Exact<{
  ft_id: Scalars['String']['input'];
  want_deltas: Scalars['Boolean']['input'];
}>;


export type MessagesSubscriptionSubscription = { __typename?: 'Subscription', comprehensive_thread_subs: { __typename?: 'FThreadMessageSubs', news_action: string, news_payload_id: string, news_payload_thread_message?: { __typename?: 'FThreadMessageOutput', ftm_belongs_to_ft_id: string, ftm_alt: number, ftm_num: number, ftm_prev_alt: number, ftm_role: string, ftm_content?: any | null, ftm_tool_calls?: any | null, ftm_call_id: string, ftm_usage?: any | null, ftm_created_ts: number } | null, stream_delta?: { __typename?: 'FThreadDelta', ftm_role: string, ftm_content: any } | null, news_payload_thread?: { __typename?: 'FThreadOutput', located_fgroup_id: string, ft_id: string } | null } };

export type MessageCreateMutationVariables = Exact<{
  input: FThreadMessageInput;
}>;


export type MessageCreateMutation = { __typename?: 'Mutation', thread_message_create: { __typename?: 'FThreadMessageOutput', ftm_belongs_to_ft_id: string, ftm_alt: number, ftm_num: number, ftm_prev_alt: number, ftm_role: string, ftm_content?: any | null, ftm_tool_calls?: any | null, ftm_call_id: string } };


export const CreateGroupDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateGroup"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fgroup_name"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fgroup_parent_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"group_create"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"fgroup_name"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fgroup_name"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"fgroup_parent_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fgroup_parent_id"}}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fgroup_id"}},{"kind":"Field","name":{"kind":"Name","value":"fgroup_name"}},{"kind":"Field","name":{"kind":"Name","value":"ws_id"}},{"kind":"Field","name":{"kind":"Name","value":"fgroup_parent_id"}},{"kind":"Field","name":{"kind":"Name","value":"fgroup_created_ts"}}]}}]}}]} as unknown as DocumentNode<CreateGroupMutation, CreateGroupMutationVariables>;
export const NavTreeSubsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"NavTreeSubs"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ws_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"tree_subscription"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"ws_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ws_id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"treeupd_action"}},{"kind":"Field","name":{"kind":"Name","value":"treeupd_id"}},{"kind":"Field","name":{"kind":"Name","value":"treeupd_path"}},{"kind":"Field","name":{"kind":"Name","value":"treeupd_type"}},{"kind":"Field","name":{"kind":"Name","value":"treeupd_title"}}]}}]}}]} as unknown as DocumentNode<NavTreeSubsSubscription, NavTreeSubsSubscriptionVariables>;
export const NavTreeWantWorkspacesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"NavTreeWantWorkspaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"query_basic_stuff"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fuser_id"}},{"kind":"Field","name":{"kind":"Name","value":"workspaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ws_id"}},{"kind":"Field","name":{"kind":"Name","value":"ws_owner_fuser_id"}},{"kind":"Field","name":{"kind":"Name","value":"ws_root_group_id"}},{"kind":"Field","name":{"kind":"Name","value":"root_group_name"}}]}}]}}]}}]} as unknown as DocumentNode<NavTreeWantWorkspacesQuery, NavTreeWantWorkspacesQueryVariables>;
export const ThreadsPageSubsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"ThreadsPageSubs"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"located_fgroup_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"limit"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"threads_in_group"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"located_fgroup_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"located_fgroup_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"limit"},"value":{"kind":"Variable","name":{"kind":"Name","value":"limit"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"news_action"}},{"kind":"Field","name":{"kind":"Name","value":"news_payload_id"}},{"kind":"Field","name":{"kind":"Name","value":"news_payload"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"owner_fuser_id"}},{"kind":"Field","name":{"kind":"Name","value":"owner_shared"}},{"kind":"Field","name":{"kind":"Name","value":"ft_id"}},{"kind":"Field","name":{"kind":"Name","value":"ft_title"}},{"kind":"Field","name":{"kind":"Name","value":"ft_error"}},{"kind":"Field","name":{"kind":"Name","value":"ft_updated_ts"}},{"kind":"Field","name":{"kind":"Name","value":"ft_locked_by"}},{"kind":"Field","name":{"kind":"Name","value":"ft_need_assistant"}},{"kind":"Field","name":{"kind":"Name","value":"ft_need_tool_calls"}},{"kind":"Field","name":{"kind":"Name","value":"ft_anything_new"}},{"kind":"Field","name":{"kind":"Name","value":"ft_archived_ts"}},{"kind":"Field","name":{"kind":"Name","value":"ft_created_ts"}},{"kind":"Field","name":{"kind":"Name","value":"ft_n"}}]}}]}}]}}]} as unknown as DocumentNode<ThreadsPageSubsSubscription, ThreadsPageSubsSubscriptionVariables>;
export const DeleteThreadDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteThread"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"thread_delete"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteThreadMutation, DeleteThreadMutationVariables>;
export const CreateThreadDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateThread"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"FThreadInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"thread_create"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ft_id"}}]}}]}}]} as unknown as DocumentNode<CreateThreadMutation, CreateThreadMutationVariables>;
export const MessagesSubscriptionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"MessagesSubscription"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ft_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"want_deltas"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Boolean"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"comprehensive_thread_subs"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"ft_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ft_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"want_deltas"},"value":{"kind":"Variable","name":{"kind":"Name","value":"want_deltas"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"news_action"}},{"kind":"Field","name":{"kind":"Name","value":"news_payload_id"}},{"kind":"Field","name":{"kind":"Name","value":"news_payload_thread_message"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ftm_belongs_to_ft_id"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_alt"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_num"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_prev_alt"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_role"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_content"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_tool_calls"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_call_id"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_usage"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_created_ts"}}]}},{"kind":"Field","name":{"kind":"Name","value":"stream_delta"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ftm_role"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_content"}}]}},{"kind":"Field","name":{"kind":"Name","value":"news_payload_thread"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"located_fgroup_id"}},{"kind":"Field","name":{"kind":"Name","value":"ft_id"}}]}}]}}]}}]} as unknown as DocumentNode<MessagesSubscriptionSubscription, MessagesSubscriptionSubscriptionVariables>;
export const MessageCreateDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"MessageCreate"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"FThreadMessageInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"thread_message_create"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ftm_belongs_to_ft_id"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_alt"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_num"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_prev_alt"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_role"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_content"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_tool_calls"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_call_id"}}]}}]}}]} as unknown as DocumentNode<MessageCreateMutation, MessageCreateMutationVariables>;

type Properties<T> = Required<{
  [K in keyof T]: z.ZodType<T[K], any, T[K]>;
}>;

type definedNonNullAny = {};

export const isDefinedNonNullAny = (v: any): v is definedNonNullAny => v !== undefined && v !== null;

export const definedNonNullAnySchema = z.any().refine((v) => isDefinedNonNullAny(v));

export function FExpertInputSchema(): z.ZodObject<Properties<FExpertInput>> {
  return z.object({
    fexp_allow_tools: z.string(),
    fexp_block_tools: z.string(),
    fexp_name: z.string(),
    fexp_python_kernel: z.string(),
    fexp_system_prompt: z.string(),
    located_fgroup_id: z.string(),
    owner_fuser_id: z.string().nullish(),
    owner_shared: z.boolean()
  })
}

export function FExpertPatchSchema(): z.ZodObject<Properties<FExpertPatch>> {
  return z.object({
    located_fgroup_id: z.string().nullish(),
    owner_shared: z.boolean().nullish()
  })
}

export function FExternalDataSourceInputSchema(): z.ZodObject<Properties<FExternalDataSourceInput>> {
  return z.object({
    eds_json: z.string(),
    eds_name: z.string(),
    eds_type: z.string(),
    located_fgroup_id: z.string()
  })
}

export function FExternalDataSourcePatchSchema(): z.ZodObject<Properties<FExternalDataSourcePatch>> {
  return z.object({
    eds_json: z.string(),
    eds_last_successful_scan_ts: z.number().nullish(),
    eds_name: z.string().nullish(),
    eds_scan_status: z.string().nullish(),
    eds_secret_id: z.number().nullish(),
    eds_type: z.string().nullish(),
    located_fgroup_id: z.string().nullish()
  })
}

export function FKnowledgeItemInputSchema(): z.ZodObject<Properties<FKnowledgeItemInput>> {
  return z.object({
    iknow_is_core: z.boolean().default(false),
    iknow_memory: z.string(),
    iknow_tags: z.array(z.string()),
    located_fgroup_id: z.string(),
    owner_shared: z.boolean()
  })
}

export function FKnowledgeItemPatchSchema(): z.ZodObject<Properties<FKnowledgeItemPatch>> {
  return z.object({
    iknow_is_core: z.boolean().nullish(),
    iknow_memory: z.string().nullish(),
    iknow_tags: z.array(z.string()).nullish(),
    located_fgroup_id: z.string().nullish(),
    owner_shared: z.boolean().nullish()
  })
}

export function FThreadInputSchema(): z.ZodObject<Properties<FThreadInput>> {
  return z.object({
    ft_app_capture: z.string().default(""),
    ft_app_searchable: z.string().default(""),
    ft_app_specific: z.string().default("null"),
    ft_belongs_to_fce_id: z.string().nullish(),
    ft_error: z.string().default("null"),
    ft_fexp_name: z.string(),
    ft_max_new_tokens: z.number().default(8192),
    ft_model: z.string().default(""),
    ft_n: z.number().default(1),
    ft_temperature: z.number().default(0),
    ft_title: z.string(),
    ft_toolset: z.string().default("null"),
    located_fgroup_id: z.string(),
    owner_shared: z.boolean()
  })
}

export function FThreadMessageInputSchema(): z.ZodObject<Properties<FThreadMessageInput>> {
  return z.object({
    ftm_alt: z.number(),
    ftm_app_specific: z.string().default("null").nullish(),
    ftm_belongs_to_ft_id: z.string(),
    ftm_call_id: z.string(),
    ftm_content: z.string().default("null").nullish(),
    ftm_num: z.number(),
    ftm_prev_alt: z.number(),
    ftm_provenance: z.string(),
    ftm_role: z.string(),
    ftm_tool_calls: z.string().default("null").nullish(),
    ftm_usage: z.string().default("null").nullish()
  })
}

export function FThreadMultipleMessagesInputSchema(): z.ZodObject<Properties<FThreadMultipleMessagesInput>> {
  return z.object({
    ftm_belongs_to_ft_id: z.string(),
    messages: z.array(z.lazy(() => FThreadMessageInputSchema()))
  })
}

export function FThreadPatchSchema(): z.ZodObject<Properties<FThreadPatch>> {
  return z.object({
    ft_anything_new: z.boolean().nullish(),
    ft_app_searchable: z.string().nullish(),
    ft_app_specific: z.string().nullish(),
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
    owner_shared: z.boolean().nullish()
  })
}

export function FlexusGroupInputSchema(): z.ZodObject<Properties<FlexusGroupInput>> {
  return z.object({
    fgroup_name: z.string(),
    fgroup_parent_id: z.string()
  })
}

export function FlexusGroupPatchSchema(): z.ZodObject<Properties<FlexusGroupPatch>> {
  return z.object({
    fgroup_name: z.string().nullish(),
    fgroup_parent_id: z.string().nullish()
  })
}

export function TechSupportSettingsInputSchema(): z.ZodObject<Properties<TechSupportSettingsInput>> {
  return z.object({
    support_api_key: z.string(),
    support_channel_list: z.array(z.string()),
    support_discord_key: z.string(),
    support_fgroup_id: z.string(),
    support_fuser_id: z.string()
  })
}
