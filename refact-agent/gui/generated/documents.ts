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
  Union: { input: any; output: any; }
};

export type BasicStuffResult = {
  __typename?: 'BasicStuffResult';
  fuser_id: Scalars['String']['output'];
  fuser_psystem?: Maybe<Scalars['JSON']['output']>;
  invitations?: Maybe<Array<FWorkspaceInvitationOutput>>;
  is_oauth: Scalars['Boolean']['output'];
  my_own_ws_id?: Maybe<Scalars['String']['output']>;
  workspaces: Array<FWorkspaceOutput>;
};

export type CloudtoolResultInput = {
  dollars?: Scalars['Float']['input'];
  fcall_id: Scalars['String']['input'];
  ftm_content: Scalars['String']['input'];
  ftm_provenance: Scalars['String']['input'];
};

export type EmailConfirmResult = {
  __typename?: 'EmailConfirmResult';
  fuser_id: Scalars['String']['output'];
};

export type FApiKeyOutput = {
  __typename?: 'FApiKeyOutput';
  apikey_archived_ts: Scalars['Float']['output'];
  apikey_created_ts: Scalars['Float']['output'];
  apikey_id: Scalars['String']['output'];
  apikey_last4digits: Scalars['String']['output'];
  full_key_shown_once?: Maybe<Scalars['String']['output']>;
};

export type FAuditRecordOutput = {
  __typename?: 'FAuditRecordOutput';
  audit_counter: Scalars['Int']['output'];
  audit_fgroup_id?: Maybe<Scalars['String']['output']>;
  audit_fuser_id?: Maybe<Scalars['String']['output']>;
  audit_metadata?: Maybe<Scalars['JSON']['output']>;
  audit_op: Scalars['String']['output'];
  audit_payload_existing_json?: Maybe<Scalars['JSON']['output']>;
  audit_payload_id: Scalars['String']['output'];
  audit_payload_updated_json?: Maybe<Scalars['JSON']['output']>;
  audit_session_id?: Maybe<Scalars['String']['output']>;
  audit_table_name: Scalars['String']['output'];
  audit_ts: Scalars['Float']['output'];
};

export type FBotInstallOutput = {
  __typename?: 'FBotInstallOutput';
  marketable_name: Scalars['String']['output'];
  marketable_version: Scalars['String']['output'];
};

export type FCloudTool = {
  __typename?: 'FCloudTool';
  ctool_confirmed_exists_ts?: Maybe<Scalars['Float']['output']>;
  ctool_description: Scalars['String']['output'];
  ctool_id: Scalars['String']['output'];
  ctool_name: Scalars['String']['output'];
  ctool_parameters: Scalars['JSON']['output'];
  located_fgroup_id?: Maybe<Scalars['String']['output']>;
  owner_fuser_id?: Maybe<Scalars['String']['output']>;
};

export type FEphemeralDocumentOutput = {
  __typename?: 'FEphemeralDocumentOutput';
  edoc_icon: Scalars['String']['output'];
  edoc_id: Scalars['String']['output'];
  edoc_mtime: Scalars['Int']['output'];
  edoc_size_bytes: Scalars['Int']['output'];
  edoc_status_download: Scalars['String']['output'];
  edoc_status_graphdb: Scalars['String']['output'];
  edoc_status_vectordb: Scalars['String']['output'];
  edoc_title: Scalars['String']['output'];
  eds_id: Scalars['String']['output'];
  eds_type: Scalars['String']['output'];
  ws_id: Scalars['String']['output'];
};

export type FEphemeralSubs = {
  __typename?: 'FEphemeralSubs';
  news_action: Scalars['String']['output'];
  news_payload?: Maybe<FEphemeralDocumentOutput>;
  news_payload_id: Scalars['String']['output'];
};

export type FExpertChoiceConsequences = {
  __typename?: 'FExpertChoiceConsequences';
  cloudtools: Array<FCloudTool>;
  models: Array<FModelItem>;
};

export type FExpertInput = {
  fexp_allow_tools: Scalars['String']['input'];
  fexp_app_capture_tools?: Scalars['String']['input'];
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
  fexp_app_capture_tools?: Maybe<Scalars['JSON']['output']>;
  fexp_block_tools: Scalars['String']['output'];
  fexp_id: Scalars['String']['output'];
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
  eds_scan_problem: Scalars['String']['output'];
  eds_type: Scalars['String']['output'];
  located_fgroup_id: Scalars['String']['output'];
  owner_fuser_id: Scalars['String']['output'];
};

export type FExternalDataSourcePatch = {
  eds_json: Scalars['String']['input'];
  eds_last_successful_scan_ts?: InputMaybe<Scalars['Float']['input']>;
  eds_name?: InputMaybe<Scalars['String']['input']>;
  located_fgroup_id?: InputMaybe<Scalars['String']['input']>;
};

export type FExternalDataSourceSubs = {
  __typename?: 'FExternalDataSourceSubs';
  news_action: Scalars['String']['output'];
  news_payload?: Maybe<FExternalDataSourceOutput>;
  news_payload_id: Scalars['String']['output'];
};

export type FKanbanTaskInput = {
  details_json?: InputMaybe<Scalars['String']['input']>;
  state: Scalars['String']['input'];
  title: Scalars['String']['input'];
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
  iknow_embedding_error: Scalars['String']['output'];
  iknow_embedding_started_ts: Scalars['Float']['output'];
  iknow_embedding_status: Scalars['String']['output'];
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

export type FMarketplaceExpertInput = {
  fexp_allow_tools: Scalars['String']['input'];
  fexp_app_capture_tools?: Scalars['String']['input'];
  fexp_block_tools: Scalars['String']['input'];
  fexp_name: Scalars['String']['input'];
  fexp_python_kernel: Scalars['String']['input'];
  fexp_system_prompt: Scalars['String']['input'];
};

export type FMarketplaceInstallOutput = {
  __typename?: 'FMarketplaceInstallOutput';
  persona_id: Scalars['String']['output'];
};

export type FMarketplaceOutput = {
  __typename?: 'FMarketplaceOutput';
  available_ws_id?: Maybe<Scalars['String']['output']>;
  marketable_description: Scalars['String']['output'];
  marketable_name: Scalars['String']['output'];
  marketable_picture_big?: Maybe<Scalars['String']['output']>;
  marketable_picture_small?: Maybe<Scalars['String']['output']>;
  marketable_popularity_counter: Scalars['Int']['output'];
  marketable_price: Scalars['Int']['output'];
  marketable_star_event: Scalars['Int']['output'];
  marketable_star_sum: Scalars['Int']['output'];
  marketable_title1: Scalars['String']['output'];
  marketable_title2: Scalars['String']['output'];
  marketable_version: Scalars['String']['output'];
  seller_fuser_id?: Maybe<Scalars['String']['output']>;
};

export type FMassInvitationOutput = {
  __typename?: 'FMassInvitationOutput';
  fuser_id: Scalars['String']['output'];
  result: Scalars['String']['output'];
};

export type FMcpServerInput = {
  located_fgroup_id: Scalars['String']['input'];
  mcp_command: Scalars['String']['input'];
  mcp_description?: Scalars['String']['input'];
  mcp_enabled?: Scalars['Boolean']['input'];
  mcp_env_vars?: InputMaybe<Scalars['JSON']['input']>;
  mcp_name: Scalars['String']['input'];
};

export type FMcpServerOutput = {
  __typename?: 'FMcpServerOutput';
  located_fgroup_id: Scalars['String']['output'];
  mcp_command: Scalars['String']['output'];
  mcp_created_ts: Scalars['Float']['output'];
  mcp_description: Scalars['String']['output'];
  mcp_enabled: Scalars['Boolean']['output'];
  mcp_env_vars?: Maybe<Scalars['JSON']['output']>;
  mcp_id: Scalars['String']['output'];
  mcp_modified_ts: Scalars['Float']['output'];
  mcp_name: Scalars['String']['output'];
  owner_fuser_id: Scalars['String']['output'];
  owner_shared: Scalars['Boolean']['output'];
};

export type FMcpServerPatch = {
  located_fgroup_id?: InputMaybe<Scalars['String']['input']>;
  mcp_command?: InputMaybe<Scalars['String']['input']>;
  mcp_description?: InputMaybe<Scalars['String']['input']>;
  mcp_enabled?: InputMaybe<Scalars['Boolean']['input']>;
  mcp_env_vars?: InputMaybe<Scalars['JSON']['input']>;
  mcp_name?: InputMaybe<Scalars['String']['input']>;
  owner_shared?: InputMaybe<Scalars['Boolean']['input']>;
};

export type FMcpServerSubs = {
  __typename?: 'FMcpServerSubs';
  news_action: Scalars['String']['output'];
  news_payload?: Maybe<FMcpServerOutput>;
  news_payload_id: Scalars['String']['output'];
  news_pubsub: Scalars['String']['output'];
};

export type FModelItem = {
  __typename?: 'FModelItem';
  provm_name: Scalars['String']['output'];
};

export type FPermissionOutput = {
  __typename?: 'FPermissionOutput';
  fgroup_id: Scalars['String']['output'];
  fuser_id: Scalars['String']['output'];
  perm_roles: Scalars['Int']['output'];
};

export type FPermissionPatch = {
  perm_roles: Scalars['Int']['input'];
};

export type FPermissionSubs = {
  __typename?: 'FPermissionSubs';
  news_action: Scalars['String']['output'];
  news_payload?: Maybe<FPermissionOutput>;
  news_payload_id: Scalars['String']['output'];
  news_pubsub: Scalars['String']['output'];
};

export type FPersonaHistoryItemOutput = {
  __typename?: 'FPersonaHistoryItemOutput';
  ft_id: Scalars['String']['output'];
  title: Scalars['String']['output'];
};

export type FPersonaInput = {
  located_fgroup_id: Scalars['String']['input'];
  persona_discounts?: InputMaybe<Scalars['String']['input']>;
  persona_marketable_name: Scalars['String']['input'];
  persona_marketable_version: Scalars['String']['input'];
  persona_name: Scalars['String']['input'];
  persona_setup: Scalars['String']['input'];
};

export type FPersonaKanbanSubs = {
  __typename?: 'FPersonaKanbanSubs';
  bucket: Scalars['String']['output'];
  news_action: Scalars['String']['output'];
  news_payload_id: Scalars['String']['output'];
  news_payload_task?: Maybe<FPersonaKanbanTaskOutput>;
};

export type FPersonaKanbanTaskOutput = {
  __typename?: 'FPersonaKanbanTaskOutput';
  ktask_blocks_ktask_id?: Maybe<Scalars['String']['output']>;
  ktask_budget: Scalars['Int']['output'];
  ktask_details: Scalars['JSON']['output'];
  ktask_done_ts: Scalars['Float']['output'];
  ktask_failed_ts: Scalars['Float']['output'];
  ktask_id: Scalars['String']['output'];
  ktask_inbox_provenance: Scalars['JSON']['output'];
  ktask_inbox_ts: Scalars['Float']['output'];
  ktask_inprogress_ft_id: Scalars['String']['output'];
  ktask_inprogress_ts: Scalars['Float']['output'];
  ktask_title: Scalars['String']['output'];
  ktask_todo_ts: Scalars['Float']['output'];
  persona_id: Scalars['String']['output'];
};

export type FPersonaOutput = {
  __typename?: 'FPersonaOutput';
  history?: Maybe<Array<FPersonaHistoryItemOutput>>;
  latest_ft_id?: Maybe<Scalars['String']['output']>;
  located_fgroup_id: Scalars['String']['output'];
  marketable_docker_image?: Maybe<Scalars['String']['output']>;
  marketable_run_this?: Maybe<Scalars['String']['output']>;
  marketable_setup_default?: Maybe<Scalars['JSON']['output']>;
  owner_fuser_id: Scalars['String']['output'];
  persona_archived_ts: Scalars['Float']['output'];
  persona_created_ts: Scalars['Float']['output'];
  persona_discounts?: Maybe<Scalars['JSON']['output']>;
  persona_enabled: Scalars['Boolean']['output'];
  persona_id: Scalars['String']['output'];
  persona_marketable_name: Scalars['String']['output'];
  persona_marketable_version: Scalars['String']['output'];
  persona_name: Scalars['String']['output'];
  persona_picture_big?: Maybe<Scalars['String']['output']>;
  persona_picture_small?: Maybe<Scalars['String']['output']>;
  persona_setup: Scalars['JSON']['output'];
};

export type FPersonaPatch = {
  located_fgroup_id?: InputMaybe<Scalars['String']['input']>;
  persona_archived_ts?: InputMaybe<Scalars['Float']['input']>;
  persona_enabled?: InputMaybe<Scalars['Boolean']['input']>;
  persona_marketable_version?: InputMaybe<Scalars['String']['input']>;
  persona_name?: InputMaybe<Scalars['String']['input']>;
  persona_setup?: InputMaybe<Scalars['String']['input']>;
};

export type FPersonaSubs = {
  __typename?: 'FPersonaSubs';
  news_action: Scalars['String']['output'];
  news_payload?: Maybe<FPersonaOutput>;
  news_payload_id: Scalars['String']['output'];
  news_pubsub: Scalars['String']['output'];
};

export type FStatsAddInput = {
  fgroup_id?: Scalars['String']['input'];
  st_chart: Scalars['Int']['input'];
  st_how_many: Scalars['Union']['input'];
  st_involved_fexp_id?: Scalars['String']['input'];
  st_involved_fuser_id?: Scalars['String']['input'];
  st_involved_model?: Scalars['String']['input'];
  st_thing: Scalars['String']['input'];
  ws_id: Scalars['String']['input'];
};

export type FStatsOutput = {
  __typename?: 'FStatsOutput';
  fgroup_id?: Maybe<Scalars['String']['output']>;
  st_how_many: Scalars['Union']['output'];
  st_involved_fexp_id?: Maybe<Scalars['String']['output']>;
  st_involved_fuser_id?: Maybe<Scalars['String']['output']>;
  st_involved_model?: Maybe<Scalars['String']['output']>;
  st_timekey: Scalars['String']['output'];
  ws_id?: Maybe<Scalars['String']['output']>;
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
  ft_error?: Scalars['String']['input'];
  ft_fexp_id: Scalars['String']['input'];
  ft_persona_id?: InputMaybe<Scalars['String']['input']>;
  ft_subchat_dest_ft_id?: InputMaybe<Scalars['String']['input']>;
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
  ftm_user_preferences?: InputMaybe<Scalars['String']['input']>;
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
  ftm_user_preferences?: Maybe<Scalars['JSON']['output']>;
};

export type FThreadMessageSubs = {
  __typename?: 'FThreadMessageSubs';
  news_action: Scalars['String']['output'];
  news_payload_id: Scalars['String']['output'];
  news_payload_thread?: Maybe<FThreadOutput>;
  news_payload_thread_message?: Maybe<FThreadMessageOutput>;
  stream_delta?: Maybe<FThreadDelta>;
};

export type FThreadMultipleMessagesInput = {
  ftm_belongs_to_ft_id: Scalars['String']['input'];
  messages: Array<FThreadMessageInput>;
};

export type FThreadOutput = {
  __typename?: 'FThreadOutput';
  ft_app_capture: Scalars['String']['output'];
  ft_app_searchable: Scalars['String']['output'];
  ft_app_specific?: Maybe<Scalars['JSON']['output']>;
  ft_archived_ts: Scalars['Float']['output'];
  ft_confirmation_request?: Maybe<Scalars['JSON']['output']>;
  ft_confirmation_response?: Maybe<Scalars['JSON']['output']>;
  ft_created_ts: Scalars['Float']['output'];
  ft_error?: Maybe<Scalars['JSON']['output']>;
  ft_fexp_id: Scalars['String']['output'];
  ft_id: Scalars['String']['output'];
  ft_locked_by: Scalars['String']['output'];
  ft_need_assistant: Scalars['Int']['output'];
  ft_need_tool_calls: Scalars['Int']['output'];
  ft_need_user: Scalars['Int']['output'];
  ft_persona_id?: Maybe<Scalars['String']['output']>;
  ft_subchat_dest_ft_id?: Maybe<Scalars['String']['output']>;
  ft_title: Scalars['String']['output'];
  ft_toolset?: Maybe<Scalars['JSON']['output']>;
  ft_updated_ts: Scalars['Float']['output'];
  located_fgroup_id: Scalars['String']['output'];
  owner_fuser_id: Scalars['String']['output'];
  owner_shared: Scalars['Boolean']['output'];
};

export type FThreadPatch = {
  ft_app_searchable?: InputMaybe<Scalars['String']['input']>;
  ft_app_specific?: InputMaybe<Scalars['String']['input']>;
  ft_archived_ts?: InputMaybe<Scalars['Float']['input']>;
  ft_confirmation_request?: InputMaybe<Scalars['String']['input']>;
  ft_confirmation_response?: InputMaybe<Scalars['String']['input']>;
  ft_error?: InputMaybe<Scalars['String']['input']>;
  ft_subchat_dest_ft_id?: InputMaybe<Scalars['String']['input']>;
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

export type FUserProfileOutput = {
  __typename?: 'FUserProfileOutput';
  fuser_experimental: Scalars['Boolean']['output'];
  fuser_fullname: Scalars['String']['output'];
  fuser_id: Scalars['String']['output'];
};

export type FUserProfilePatch = {
  fuser_experimental?: InputMaybe<Scalars['Boolean']['input']>;
  fuser_fullname?: InputMaybe<Scalars['String']['input']>;
};

export type FWorkspaceCreateInput = {
  ws_name: Scalars['String']['input'];
};

export type FWorkspaceInvitationOutput = {
  __typename?: 'FWorkspaceInvitationOutput';
  group_name: Scalars['String']['output'];
  wsi_fgroup_id: Scalars['String']['output'];
  wsi_id: Scalars['String']['output'];
  wsi_invite_fuser_id: Scalars['String']['output'];
  wsi_invited_by_fuser_id: Scalars['String']['output'];
  wsi_roles: Scalars['Int']['output'];
};

export type FWorkspaceOutput = {
  __typename?: 'FWorkspaceOutput';
  have_admin: Scalars['Boolean']['output'];
  have_coins_enough: Scalars['Boolean']['output'];
  have_coins_exactly: Scalars['Union']['output'];
  root_group_name: Scalars['String']['output'];
  ws_archived_ts: Scalars['Float']['output'];
  ws_created_ts: Scalars['Float']['output'];
  ws_id: Scalars['String']['output'];
  ws_owner_fuser_id: Scalars['String']['output'];
  ws_root_group_id: Scalars['String']['output'];
};

export type FlexusGroup = {
  __typename?: 'FlexusGroup';
  fgroup_created_ts: Scalars['Float']['output'];
  fgroup_id: Scalars['String']['output'];
  fgroup_name: Scalars['String']['output'];
  fgroup_parent_id?: Maybe<Scalars['String']['output']>;
  my_roles?: Maybe<Scalars['Int']['output']>;
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
  api_key_delete: Scalars['Boolean']['output'];
  api_key_generate: FApiKeyOutput;
  bot_activate: FThreadOutput;
  bot_arrange_kanban_situation: Scalars['Boolean']['output'];
  bot_install_from_marketplace: Scalars['Boolean']['output'];
  bot_kanban_post_into_inbox: Scalars['Boolean']['output'];
  cloudtool_post_result: Scalars['Boolean']['output'];
  create_captured_thread: FThreadOutput;
  email_confirm: EmailConfirmResult;
  expert_create: FExpertOutput;
  expert_delete: Scalars['Boolean']['output'];
  expert_patch: FExpertOutput;
  external_data_source_create: FExternalDataSourceOutput;
  external_data_source_delete: Scalars['Boolean']['output'];
  external_data_source_patch: FExternalDataSourceOutput;
  group_create: FlexusGroup;
  group_delete: Scalars['String']['output'];
  group_patch: FlexusGroup;
  invitation_accept: Scalars['Boolean']['output'];
  invitation_create_multiple: Array<FMassInvitationOutput>;
  invitation_delete: Scalars['Boolean']['output'];
  invitation_reject: Scalars['Boolean']['output'];
  knowledge_item_create: FKnowledgeItemOutput;
  knowledge_item_delete: Scalars['Boolean']['output'];
  knowledge_item_mass_group_patch: Scalars['Int']['output'];
  knowledge_item_patch: FKnowledgeItemOutput;
  make_sure_have_expert: Scalars['String']['output'];
  marketplace_install: FMarketplaceInstallOutput;
  marketplace_upgrade: Scalars['Boolean']['output'];
  marketplace_upsert_dev_bot: FBotInstallOutput;
  mcp_server_create: FMcpServerOutput;
  mcp_server_delete: Scalars['Boolean']['output'];
  mcp_server_patch: FMcpServerOutput;
  password_change: Scalars['Boolean']['output'];
  permission_delete: Scalars['Boolean']['output'];
  permission_patch: FPermissionOutput;
  persona_create: FPersonaOutput;
  persona_delete: Scalars['Boolean']['output'];
  persona_patch: FPersonaOutput;
  reset_password_execute: Scalars['Boolean']['output'];
  reset_password_start: Scalars['Boolean']['output'];
  session_open: Scalars['String']['output'];
  session_renew: Scalars['String']['output'];
  stats_add: Scalars['Boolean']['output'];
  thread_app_capture_patch: Scalars['Boolean']['output'];
  thread_clear_confirmation: Scalars['Boolean']['output'];
  thread_create: FThreadOutput;
  thread_delete: Scalars['Boolean']['output'];
  thread_lock: Scalars['Boolean']['output'];
  thread_mass_group_patch: Scalars['Int']['output'];
  thread_messages_create_multiple: Scalars['Int']['output'];
  thread_patch: FThreadOutput;
  thread_reset_error: Scalars['Boolean']['output'];
  thread_reset_title: Scalars['Boolean']['output'];
  thread_set_confirmation_request: Scalars['Boolean']['output'];
  thread_set_confirmation_response: Scalars['Boolean']['output'];
  thread_unlock: Scalars['Boolean']['output'];
  user_profile_patch: FUserProfileOutput;
  user_register: Scalars['Boolean']['output'];
  workspace_create: Scalars['String']['output'];
  workspace_delete: Scalars['String']['output'];
  workspace_leave: Scalars['String']['output'];
};


export type MutationApi_Key_DeleteArgs = {
  apikey_id: Scalars['String']['input'];
};


export type MutationBot_ActivateArgs = {
  activation_type: Scalars['String']['input'];
  first_calls: Scalars['String']['input'];
  first_question: Scalars['String']['input'];
  localtools: Scalars['String']['input'];
  persona_id: Scalars['String']['input'];
  title: Scalars['String']['input'];
  who_is_asking: Scalars['String']['input'];
};


export type MutationBot_Arrange_Kanban_SituationArgs = {
  persona_id: Scalars['String']['input'];
  tasks: Array<FKanbanTaskInput>;
  ws_id: Scalars['String']['input'];
};


export type MutationBot_Install_From_MarketplaceArgs = {
  inside_fgroup_id: Scalars['String']['input'];
  new_setup: Scalars['String']['input'];
  persona_id: Scalars['String']['input'];
  persona_marketable_name: Scalars['String']['input'];
  persona_marketable_version: Scalars['String']['input'];
  persona_name: Scalars['String']['input'];
};


export type MutationBot_Kanban_Post_Into_InboxArgs = {
  budget: Scalars['Int']['input'];
  details_json: Scalars['String']['input'];
  persona_id: Scalars['String']['input'];
  title: Scalars['String']['input'];
};


export type MutationCloudtool_Post_ResultArgs = {
  input: CloudtoolResultInput;
};


export type MutationCreate_Captured_ThreadArgs = {
  input: FThreadInput;
  on_behalf_of_fuser_id?: InputMaybe<Scalars['String']['input']>;
};


export type MutationEmail_ConfirmArgs = {
  token: Scalars['String']['input'];
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


export type MutationInvitation_AcceptArgs = {
  wsi_id: Scalars['String']['input'];
};


export type MutationInvitation_Create_MultipleArgs = {
  emails: Array<Scalars['String']['input']>;
  fgroup_id: Scalars['String']['input'];
  roles: Scalars['Int']['input'];
};


export type MutationInvitation_DeleteArgs = {
  wsi_fgroup_id: Scalars['String']['input'];
  wsi_invite_fuser_id: Scalars['String']['input'];
};


export type MutationInvitation_RejectArgs = {
  wsi_id: Scalars['String']['input'];
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


export type MutationMake_Sure_Have_ExpertArgs = {
  fexp_name: Scalars['String']['input'];
  fgroup_id?: InputMaybe<Scalars['String']['input']>;
  owner_fuser_id?: InputMaybe<Scalars['String']['input']>;
  python_kernel: Scalars['String']['input'];
  system_prompt: Scalars['String']['input'];
};


export type MutationMarketplace_InstallArgs = {
  fgroup_id: Scalars['String']['input'];
  marketable_name: Scalars['String']['input'];
};


export type MutationMarketplace_UpgradeArgs = {
  fgroup_id: Scalars['String']['input'];
  marketable_name: Scalars['String']['input'];
  specific_version: Scalars['String']['input'];
};


export type MutationMarketplace_Upsert_Dev_BotArgs = {
  marketable_description: Scalars['String']['input'];
  marketable_expert_default: FMarketplaceExpertInput;
  marketable_expert_setup?: InputMaybe<FMarketplaceExpertInput>;
  marketable_expert_subchat?: InputMaybe<FMarketplaceExpertInput>;
  marketable_expert_todo?: InputMaybe<FMarketplaceExpertInput>;
  marketable_github_repo: Scalars['String']['input'];
  marketable_name: Scalars['String']['input'];
  marketable_picture_big_b64?: InputMaybe<Scalars['String']['input']>;
  marketable_picture_small_b64?: InputMaybe<Scalars['String']['input']>;
  marketable_run_this: Scalars['String']['input'];
  marketable_setup_default: Scalars['String']['input'];
  marketable_title1: Scalars['String']['input'];
  marketable_title2: Scalars['String']['input'];
  marketable_version: Scalars['String']['input'];
  ws_id: Scalars['String']['input'];
};


export type MutationMcp_Server_CreateArgs = {
  input: FMcpServerInput;
};


export type MutationMcp_Server_DeleteArgs = {
  id: Scalars['String']['input'];
};


export type MutationMcp_Server_PatchArgs = {
  id: Scalars['String']['input'];
  patch: FMcpServerPatch;
};


export type MutationPassword_ChangeArgs = {
  new_password: Scalars['String']['input'];
  old_password: Scalars['String']['input'];
};


export type MutationPermission_DeleteArgs = {
  fgroup_id: Scalars['String']['input'];
  fuser_id: Scalars['String']['input'];
};


export type MutationPermission_PatchArgs = {
  fgroup_id: Scalars['String']['input'];
  fuser_id: Scalars['String']['input'];
  patch: FPermissionPatch;
};


export type MutationPersona_CreateArgs = {
  input: FPersonaInput;
};


export type MutationPersona_DeleteArgs = {
  id: Scalars['String']['input'];
};


export type MutationPersona_PatchArgs = {
  id: Scalars['String']['input'];
  patch: FPersonaPatch;
};


export type MutationReset_Password_ExecuteArgs = {
  new_password: Scalars['String']['input'];
  token: Scalars['String']['input'];
};


export type MutationReset_Password_StartArgs = {
  username: Scalars['String']['input'];
};


export type MutationSession_OpenArgs = {
  password: Scalars['String']['input'];
  username: Scalars['String']['input'];
};


export type MutationStats_AddArgs = {
  records: Array<FStatsAddInput>;
};


export type MutationThread_App_Capture_PatchArgs = {
  ft_app_searchable?: InputMaybe<Scalars['String']['input']>;
  ft_app_specific?: InputMaybe<Scalars['String']['input']>;
  ft_id: Scalars['String']['input'];
};


export type MutationThread_Clear_ConfirmationArgs = {
  ft_id: Scalars['String']['input'];
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


export type MutationThread_Messages_Create_MultipleArgs = {
  delete_negative?: InputMaybe<Array<Scalars['Int']['input']>>;
  input: FThreadMultipleMessagesInput;
};


export type MutationThread_PatchArgs = {
  id: Scalars['String']['input'];
  patch: FThreadPatch;
};


export type MutationThread_Reset_ErrorArgs = {
  ft_error: Scalars['String']['input'];
  ft_id: Scalars['String']['input'];
};


export type MutationThread_Reset_TitleArgs = {
  ft_id: Scalars['String']['input'];
  ft_title: Scalars['String']['input'];
};


export type MutationThread_Set_Confirmation_RequestArgs = {
  confirmation_request: Scalars['String']['input'];
  ft_id: Scalars['String']['input'];
};


export type MutationThread_Set_Confirmation_ResponseArgs = {
  confirmation_response: Scalars['String']['input'];
  ft_id: Scalars['String']['input'];
};


export type MutationThread_UnlockArgs = {
  ft_id: Scalars['String']['input'];
  worker_name: Scalars['String']['input'];
};


export type MutationUser_Profile_PatchArgs = {
  patch: FUserProfilePatch;
};


export type MutationUser_RegisterArgs = {
  input: RegisterInput;
};


export type MutationWorkspace_CreateArgs = {
  input: FWorkspaceCreateInput;
};


export type MutationWorkspace_DeleteArgs = {
  dry_run?: Scalars['Boolean']['input'];
  ws_id: Scalars['String']['input'];
};


export type MutationWorkspace_LeaveArgs = {
  ws_id: Scalars['String']['input'];
};

export type PasswordResetTokenInfo = {
  __typename?: 'PasswordResetTokenInfo';
  freset_used: Scalars['Boolean']['output'];
  fuser_id: Scalars['String']['output'];
};

export type Query = {
  __typename?: 'Query';
  api_key_list: Array<FApiKeyOutput>;
  audit_list: Array<FAuditRecordOutput>;
  cloud_tools_list: Array<FCloudTool>;
  expert_choice_consequences: FExpertChoiceConsequences;
  expert_get: FExpertOutput;
  expert_list: Array<FExpertOutput>;
  experts_effective_list: Array<FExpertOutput>;
  external_data_source_get: FExternalDataSourceOutput;
  external_data_source_list: Array<FExternalDataSourceOutput>;
  group_get: FlexusGroup;
  group_list_for_workspace: Array<FlexusGroup>;
  invitation_list: Array<FWorkspaceInvitationOutput>;
  knowledge_get_cores: Array<FKnowledgeItemOutput>;
  knowledge_item_get: FKnowledgeItemOutput;
  knowledge_item_list: Array<FKnowledgeItemOutput>;
  knowledge_vecdb_search: Array<FKnowledgeItemOutput>;
  marketplace_details: Array<FMarketplaceOutput>;
  marketplace_list: Array<FMarketplaceOutput>;
  marketplace_search: Array<FMarketplaceOutput>;
  mcp_server_get: FMcpServerOutput;
  mcp_server_list: Array<FMcpServerOutput>;
  permission_list: Array<FPermissionOutput>;
  persona_get: FPersonaOutput;
  persona_list: Array<FPersonaOutput>;
  persona_opened_in_ui: FPersonaOutput;
  query_basic_stuff: BasicStuffResult;
  reset_password_token_info: PasswordResetTokenInfo;
  stats_query: Array<FStatsOutput>;
  stats_query_distinct: StatsDistinctOutput;
  thread_get: FThreadOutput;
  thread_list: Array<FThreadOutput>;
  thread_messages_list: Array<FThreadMessageOutput>;
  threads_app_captured: Array<FThreadOutput>;
  user_profile_get: FUserProfileOutput;
  workspace_permission_list: Array<FPermissionOutput>;
  worldmap_everything: Scalars['JSON']['output'];
};


export type QueryAudit_ListArgs = {
  limit: Scalars['Int']['input'];
  skip: Scalars['Int']['input'];
  ws_id: Scalars['String']['input'];
};


export type QueryCloud_Tools_ListArgs = {
  include_offline?: Scalars['Boolean']['input'];
  located_fgroup_id: Scalars['String']['input'];
};


export type QueryExpert_Choice_ConsequencesArgs = {
  fexp_id: Scalars['String']['input'];
  inside_fgroup_id: Scalars['String']['input'];
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


export type QueryGroup_GetArgs = {
  fgroup_id: Scalars['String']['input'];
};


export type QueryGroup_List_For_WorkspaceArgs = {
  ws_id: Scalars['String']['input'];
};


export type QueryInvitation_ListArgs = {
  wsi_fgroup_id: Scalars['String']['input'];
};


export type QueryKnowledge_Get_CoresArgs = {
  fgroup_id: Scalars['String']['input'];
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


export type QueryKnowledge_Vecdb_SearchArgs = {
  fgroup_id: Scalars['String']['input'];
  q: Scalars['String']['input'];
  top_n?: Scalars['Int']['input'];
};


export type QueryMarketplace_DetailsArgs = {
  fgroup_id: Scalars['String']['input'];
  marketable_name: Scalars['String']['input'];
};


export type QueryMarketplace_ListArgs = {
  fgroup_id: Scalars['String']['input'];
  take?: Scalars['Int']['input'];
};


export type QueryMarketplace_SearchArgs = {
  fgroup_id: Scalars['String']['input'];
  query: Scalars['String']['input'];
  take?: Scalars['Int']['input'];
};


export type QueryMcp_Server_GetArgs = {
  id: Scalars['String']['input'];
};


export type QueryMcp_Server_ListArgs = {
  limit: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  skip: Scalars['Int']['input'];
  sort_by?: Scalars['String']['input'];
};


export type QueryPermission_ListArgs = {
  fgroup_id: Scalars['String']['input'];
};


export type QueryPersona_GetArgs = {
  id: Scalars['String']['input'];
};


export type QueryPersona_ListArgs = {
  limit: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  skip: Scalars['Int']['input'];
  sort_by?: Scalars['String']['input'];
};


export type QueryPersona_Opened_In_UiArgs = {
  persona_id: Scalars['String']['input'];
};


export type QueryQuery_Basic_StuffArgs = {
  want_invitations?: Scalars['Boolean']['input'];
};


export type QueryReset_Password_Token_InfoArgs = {
  token: Scalars['String']['input'];
};


export type QueryStats_QueryArgs = {
  breakdown_fexp_name: Array<Scalars['String']['input']>;
  breakdown_fuser_id: Array<Scalars['String']['input']>;
  breakdown_model: Array<Scalars['String']['input']>;
  fgroup_id?: Scalars['String']['input'];
  filter_fexp_id?: Array<Scalars['String']['input']>;
  filter_fuser_id?: Array<Scalars['String']['input']>;
  filter_model?: Array<Scalars['String']['input']>;
  filter_thing?: Array<Scalars['String']['input']>;
  st_chart: Scalars['Int']['input'];
  st_span: Scalars['String']['input'];
  timekey_from: Scalars['String']['input'];
  timekey_to: Scalars['String']['input'];
  ws_id?: Scalars['String']['input'];
};


export type QueryStats_Query_DistinctArgs = {
  fgroup_id: Scalars['String']['input'];
  filter_fexp_id: Array<Scalars['String']['input']>;
  filter_fuser_id: Array<Scalars['String']['input']>;
  filter_model: Array<Scalars['String']['input']>;
  st_chart: Scalars['Int']['input'];
  st_span: Scalars['String']['input'];
  timekey_from: Scalars['String']['input'];
  timekey_to: Scalars['String']['input'];
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


export type QueryWorkspace_Permission_ListArgs = {
  ws_id: Scalars['String']['input'];
};


export type QueryWorldmap_EverythingArgs = {
  fgroup_id: Scalars['String']['input'];
};

export type RegisterInput = {
  fullname: Scalars['String']['input'];
  password: Scalars['String']['input'];
  username: Scalars['String']['input'];
};

export type StatsDistinctOutput = {
  __typename?: 'StatsDistinctOutput';
  st_involved_fexp_id: Scalars['JSON']['output'];
  st_involved_fuser_id: Scalars['JSON']['output'];
  st_involved_model: Scalars['JSON']['output'];
  st_thing: Array<Scalars['String']['output']>;
  timekey_now: Scalars['String']['output'];
};

export type Subscription = {
  __typename?: 'Subscription';
  comprehensive_thread_subs: FThreadMessageSubs;
  ephemeral_subs: FEphemeralSubs;
  experts_in_group: FExpertSubs;
  external_data_sources_in_group: FExternalDataSourceSubs;
  knowledge_items_in_group: FKnowledgeItemSubs;
  mcp_servers_in_group: FMcpServerSubs;
  permissions_in_group_subs: FPermissionSubs;
  persona_kanban_subs: FPersonaKanbanSubs;
  personas_in_group: FPersonaSubs;
  threads_in_group: FThreadSubs;
  tree_subscription: TreeUpdateSubs;
};


export type SubscriptionComprehensive_Thread_SubsArgs = {
  ft_id: Scalars['String']['input'];
  want_deltas: Scalars['Boolean']['input'];
};


export type SubscriptionEphemeral_SubsArgs = {
  eds_id: Scalars['String']['input'];
};


export type SubscriptionExperts_In_GroupArgs = {
  filter?: Array<Scalars['String']['input']>;
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Array<Scalars['String']['input']>;
};


export type SubscriptionExternal_Data_Sources_In_GroupArgs = {
  filter?: Array<Scalars['String']['input']>;
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Array<Scalars['String']['input']>;
};


export type SubscriptionKnowledge_Items_In_GroupArgs = {
  filter?: Array<Scalars['String']['input']>;
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Array<Scalars['String']['input']>;
};


export type SubscriptionMcp_Servers_In_GroupArgs = {
  filter?: Array<Scalars['String']['input']>;
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Array<Scalars['String']['input']>;
};


export type SubscriptionPermissions_In_Group_SubsArgs = {
  fgroup_id: Scalars['String']['input'];
  limit: Scalars['Int']['input'];
  quicksearch: Scalars['String']['input'];
};


export type SubscriptionPersona_Kanban_SubsArgs = {
  limit_done?: Scalars['Int']['input'];
  limit_garbage?: Scalars['Int']['input'];
  limit_inbox?: Scalars['Int']['input'];
  persona_id: Scalars['String']['input'];
};


export type SubscriptionPersonas_In_GroupArgs = {
  filter?: Array<Scalars['String']['input']>;
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Array<Scalars['String']['input']>;
};


export type SubscriptionThreads_In_GroupArgs = {
  filter?: Array<Scalars['String']['input']>;
  limit?: Scalars['Int']['input'];
  located_fgroup_id: Scalars['String']['input'];
  sort_by?: Array<Scalars['String']['input']>;
};


export type SubscriptionTree_SubscriptionArgs = {
  ws_id: Scalars['String']['input'];
};

export type TreeUpdateSubs = {
  __typename?: 'TreeUpdateSubs';
  treeupd_action: Scalars['String']['output'];
  treeupd_id: Scalars['String']['output'];
  treeupd_path: Scalars['String']['output'];
  treeupd_roles?: Maybe<Scalars['Int']['output']>;
  treeupd_tag: Scalars['String']['output'];
  treeupd_title: Scalars['String']['output'];
  treeupd_type: Scalars['String']['output'];
};

export type ThreadsPageSubsSubscriptionVariables = Exact<{
  located_fgroup_id: Scalars['String']['input'];
  limit: Scalars['Int']['input'];
}>;


export type ThreadsPageSubsSubscription = { __typename?: 'Subscription', threads_in_group: { __typename?: 'FThreadSubs', news_action: string, news_payload_id: string, news_payload?: { __typename?: 'FThreadOutput', owner_fuser_id: string, owner_shared: boolean, ft_id: string, ft_title: string, ft_error?: any | null, ft_updated_ts: number, ft_locked_by: string, ft_need_assistant: number, ft_need_tool_calls: number, ft_archived_ts: number, ft_created_ts: number } | null } };

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


export type MessagesSubscriptionSubscription = { __typename?: 'Subscription', comprehensive_thread_subs: { __typename?: 'FThreadMessageSubs', news_action: string, news_payload_id: string, news_payload_thread_message?: { __typename?: 'FThreadMessageOutput', ft_app_specific?: any | null, ftm_belongs_to_ft_id: string, ftm_alt: number, ftm_num: number, ftm_prev_alt: number, ftm_role: string, ftm_content?: any | null, ftm_tool_calls?: any | null, ftm_call_id: string, ftm_usage?: any | null, ftm_created_ts: number, ftm_user_preferences?: any | null } | null, stream_delta?: { __typename?: 'FThreadDelta', ftm_role: string, ftm_content: any } | null, news_payload_thread?: { __typename?: 'FThreadOutput', located_fgroup_id: string, ft_id: string, ft_need_user: number, ft_need_assistant: number, ft_fexp_id: string, ft_confirmation_request?: any | null, ft_confirmation_response?: any | null, ft_title: string } | null } };

export type MessageCreateMultipleMutationVariables = Exact<{
  input: FThreadMultipleMessagesInput;
}>;


export type MessageCreateMultipleMutation = { __typename?: 'Mutation', thread_messages_create_multiple: number };

export type ThreadPatchMutationVariables = Exact<{
  id: Scalars['String']['input'];
  message: Scalars['String']['input'];
}>;


export type ThreadPatchMutation = { __typename?: 'Mutation', thread_patch: { __typename?: 'FThreadOutput', ft_id: string } };

export type ExpertsForGroupQueryVariables = Exact<{
  located_fgroup_id: Scalars['String']['input'];
}>;


export type ExpertsForGroupQuery = { __typename?: 'Query', experts_effective_list: Array<{ __typename?: 'FExpertOutput', fexp_id: string, fexp_name: string }> };

export type ModelsForExpertQueryVariables = Exact<{
  fexp_id: Scalars['String']['input'];
  inside_fgroup_id: Scalars['String']['input'];
}>;


export type ModelsForExpertQuery = { __typename?: 'Query', expert_choice_consequences: { __typename?: 'FExpertChoiceConsequences', models: Array<{ __typename?: 'FModelItem', provm_name: string }> } };

export type ToolsForGroupQueryVariables = Exact<{
  located_fgroup_id: Scalars['String']['input'];
}>;


export type ToolsForGroupQuery = { __typename?: 'Query', cloud_tools_list: Array<{ __typename?: 'FCloudTool', ctool_confirmed_exists_ts?: number | null, ctool_description: string, ctool_id: string, ctool_name: string, ctool_parameters: any, located_fgroup_id?: string | null, owner_fuser_id?: string | null }> };

export type ThreadConfirmationResponseMutationVariables = Exact<{
  confirmation_response?: InputMaybe<Scalars['String']['input']>;
  ft_id?: InputMaybe<Scalars['String']['input']>;
}>;


export type ThreadConfirmationResponseMutation = { __typename?: 'Mutation', thread_set_confirmation_response: boolean };

export type BasicStuffQueryVariables = Exact<{ [key: string]: never; }>;


export type BasicStuffQuery = { __typename?: 'Query', query_basic_stuff: { __typename?: 'BasicStuffResult', fuser_id: string, my_own_ws_id?: string | null, workspaces: Array<{ __typename?: 'FWorkspaceOutput', ws_id: string, ws_owner_fuser_id: string, ws_root_group_id: string, root_group_name: string, have_coins_exactly: any, have_coins_enough: boolean, have_admin: boolean }> } };

export type CreateWorkSpaceGroupMutationVariables = Exact<{
  fgroup_name: Scalars['String']['input'];
  fgroup_parent_id: Scalars['String']['input'];
}>;


export type CreateWorkSpaceGroupMutation = { __typename?: 'Mutation', group_create: { __typename?: 'FlexusGroup', fgroup_id: string, fgroup_name: string, ws_id: string, fgroup_parent_id?: string | null, fgroup_created_ts: number } };

export type WorkspaceTreeSubscriptionVariables = Exact<{
  ws_id: Scalars['String']['input'];
}>;


export type WorkspaceTreeSubscription = { __typename?: 'Subscription', tree_subscription: { __typename?: 'TreeUpdateSubs', treeupd_action: string, treeupd_id: string, treeupd_path: string, treeupd_type: string, treeupd_title: string } };


export const ThreadsPageSubsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"ThreadsPageSubs"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"located_fgroup_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"limit"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"threads_in_group"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"located_fgroup_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"located_fgroup_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"limit"},"value":{"kind":"Variable","name":{"kind":"Name","value":"limit"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"news_action"}},{"kind":"Field","name":{"kind":"Name","value":"news_payload_id"}},{"kind":"Field","name":{"kind":"Name","value":"news_payload"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"owner_fuser_id"}},{"kind":"Field","name":{"kind":"Name","value":"owner_shared"}},{"kind":"Field","name":{"kind":"Name","value":"ft_id"}},{"kind":"Field","name":{"kind":"Name","value":"ft_title"}},{"kind":"Field","name":{"kind":"Name","value":"ft_error"}},{"kind":"Field","name":{"kind":"Name","value":"ft_updated_ts"}},{"kind":"Field","name":{"kind":"Name","value":"ft_locked_by"}},{"kind":"Field","name":{"kind":"Name","value":"ft_need_assistant"}},{"kind":"Field","name":{"kind":"Name","value":"ft_need_tool_calls"}},{"kind":"Field","name":{"kind":"Name","value":"ft_archived_ts"}},{"kind":"Field","name":{"kind":"Name","value":"ft_created_ts"}}]}}]}}]}}]} as unknown as DocumentNode<ThreadsPageSubsSubscription, ThreadsPageSubsSubscriptionVariables>;
export const DeleteThreadDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"DeleteThread"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"thread_delete"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}]}]}}]} as unknown as DocumentNode<DeleteThreadMutation, DeleteThreadMutationVariables>;
export const CreateThreadDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateThread"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"FThreadInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"thread_create"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ft_id"}}]}}]}}]} as unknown as DocumentNode<CreateThreadMutation, CreateThreadMutationVariables>;
export const MessagesSubscriptionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"MessagesSubscription"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ft_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"want_deltas"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Boolean"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"comprehensive_thread_subs"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"ft_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ft_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"want_deltas"},"value":{"kind":"Variable","name":{"kind":"Name","value":"want_deltas"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"news_action"}},{"kind":"Field","name":{"kind":"Name","value":"news_payload_id"}},{"kind":"Field","name":{"kind":"Name","value":"news_payload_thread_message"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ft_app_specific"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_belongs_to_ft_id"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_alt"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_num"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_prev_alt"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_role"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_content"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_tool_calls"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_call_id"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_usage"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_created_ts"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_user_preferences"}}]}},{"kind":"Field","name":{"kind":"Name","value":"stream_delta"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ftm_role"}},{"kind":"Field","name":{"kind":"Name","value":"ftm_content"}}]}},{"kind":"Field","name":{"kind":"Name","value":"news_payload_thread"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"located_fgroup_id"}},{"kind":"Field","name":{"kind":"Name","value":"ft_id"}},{"kind":"Field","name":{"kind":"Name","value":"ft_need_user"}},{"kind":"Field","name":{"kind":"Name","value":"ft_need_assistant"}},{"kind":"Field","name":{"kind":"Name","value":"ft_fexp_id"}},{"kind":"Field","name":{"kind":"Name","value":"ft_confirmation_request"}},{"kind":"Field","name":{"kind":"Name","value":"ft_confirmation_response"}},{"kind":"Field","name":{"kind":"Name","value":"ft_title"}}]}}]}}]}}]} as unknown as DocumentNode<MessagesSubscriptionSubscription, MessagesSubscriptionSubscriptionVariables>;
export const MessageCreateMultipleDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"MessageCreateMultiple"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"FThreadMultipleMessagesInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"thread_messages_create_multiple"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}]}]}}]} as unknown as DocumentNode<MessageCreateMultipleMutation, MessageCreateMultipleMutationVariables>;
export const ThreadPatchDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ThreadPatch"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"message"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"thread_patch"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}},{"kind":"Argument","name":{"kind":"Name","value":"patch"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"ft_error"},"value":{"kind":"Variable","name":{"kind":"Name","value":"message"}}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ft_id"}}]}}]}}]} as unknown as DocumentNode<ThreadPatchMutation, ThreadPatchMutationVariables>;
export const ExpertsForGroupDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"ExpertsForGroup"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"located_fgroup_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"experts_effective_list"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"located_fgroup_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"located_fgroup_id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fexp_id"}},{"kind":"Field","name":{"kind":"Name","value":"fexp_name"}}]}}]}}]} as unknown as DocumentNode<ExpertsForGroupQuery, ExpertsForGroupQueryVariables>;
export const ModelsForExpertDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"ModelsForExpert"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fexp_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"inside_fgroup_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"expert_choice_consequences"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"fexp_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fexp_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"inside_fgroup_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"inside_fgroup_id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"models"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"provm_name"}}]}}]}}]}}]} as unknown as DocumentNode<ModelsForExpertQuery, ModelsForExpertQueryVariables>;
export const ToolsForGroupDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"ToolsForGroup"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"located_fgroup_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"cloud_tools_list"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"located_fgroup_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"located_fgroup_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"include_offline"},"value":{"kind":"BooleanValue","value":false}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ctool_confirmed_exists_ts"}},{"kind":"Field","name":{"kind":"Name","value":"ctool_description"}},{"kind":"Field","name":{"kind":"Name","value":"ctool_id"}},{"kind":"Field","name":{"kind":"Name","value":"ctool_name"}},{"kind":"Field","name":{"kind":"Name","value":"ctool_parameters"}},{"kind":"Field","name":{"kind":"Name","value":"located_fgroup_id"}},{"kind":"Field","name":{"kind":"Name","value":"owner_fuser_id"}}]}}]}}]} as unknown as DocumentNode<ToolsForGroupQuery, ToolsForGroupQueryVariables>;
export const ThreadConfirmationResponseDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"ThreadConfirmationResponse"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"confirmation_response"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}},"defaultValue":{"kind":"StringValue","value":"","block":false}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ft_id"}},"type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}},"defaultValue":{"kind":"StringValue","value":"","block":false}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"thread_set_confirmation_response"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"ft_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ft_id"}}},{"kind":"Argument","name":{"kind":"Name","value":"confirmation_response"},"value":{"kind":"Variable","name":{"kind":"Name","value":"confirmation_response"}}}]}]}}]} as unknown as DocumentNode<ThreadConfirmationResponseMutation, ThreadConfirmationResponseMutationVariables>;
export const BasicStuffDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"BasicStuff"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"query_basic_stuff"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fuser_id"}},{"kind":"Field","name":{"kind":"Name","value":"my_own_ws_id"}},{"kind":"Field","name":{"kind":"Name","value":"workspaces"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"ws_id"}},{"kind":"Field","name":{"kind":"Name","value":"ws_owner_fuser_id"}},{"kind":"Field","name":{"kind":"Name","value":"ws_root_group_id"}},{"kind":"Field","name":{"kind":"Name","value":"root_group_name"}},{"kind":"Field","name":{"kind":"Name","value":"have_coins_exactly"}},{"kind":"Field","name":{"kind":"Name","value":"have_coins_enough"}},{"kind":"Field","name":{"kind":"Name","value":"have_admin"}}]}}]}}]}}]} as unknown as DocumentNode<BasicStuffQuery, BasicStuffQueryVariables>;
export const CreateWorkSpaceGroupDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"CreateWorkSpaceGroup"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fgroup_name"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fgroup_parent_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"group_create"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"fgroup_name"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fgroup_name"}}},{"kind":"ObjectField","name":{"kind":"Name","value":"fgroup_parent_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fgroup_parent_id"}}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"fgroup_id"}},{"kind":"Field","name":{"kind":"Name","value":"fgroup_name"}},{"kind":"Field","name":{"kind":"Name","value":"ws_id"}},{"kind":"Field","name":{"kind":"Name","value":"fgroup_parent_id"}},{"kind":"Field","name":{"kind":"Name","value":"fgroup_created_ts"}}]}}]}}]} as unknown as DocumentNode<CreateWorkSpaceGroupMutation, CreateWorkSpaceGroupMutationVariables>;
export const WorkspaceTreeDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"WorkspaceTree"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"ws_id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"tree_subscription"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"ws_id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"ws_id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"treeupd_action"}},{"kind":"Field","name":{"kind":"Name","value":"treeupd_id"}},{"kind":"Field","name":{"kind":"Name","value":"treeupd_path"}},{"kind":"Field","name":{"kind":"Name","value":"treeupd_type"}},{"kind":"Field","name":{"kind":"Name","value":"treeupd_title"}}]}}]}}]} as unknown as DocumentNode<WorkspaceTreeSubscription, WorkspaceTreeSubscriptionVariables>;

type Properties<T> = Required<{
  [K in keyof T]: z.ZodType<T[K], any, T[K]>;
}>;

type definedNonNullAny = {};

export const isDefinedNonNullAny = (v: any): v is definedNonNullAny => v !== undefined && v !== null;

export const definedNonNullAnySchema = z.any().refine((v) => isDefinedNonNullAny(v));

export function CloudtoolResultInputSchema(): z.ZodObject<Properties<CloudtoolResultInput>> {
  return z.object({
    dollars: z.number().default(0),
    fcall_id: z.string(),
    ftm_content: z.string(),
    ftm_provenance: z.string()
  })
}

export function FExpertInputSchema(): z.ZodObject<Properties<FExpertInput>> {
  return z.object({
    fexp_allow_tools: z.string(),
    fexp_app_capture_tools: z.string().default("null"),
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
    located_fgroup_id: z.string().nullish()
  })
}

export function FKanbanTaskInputSchema(): z.ZodObject<Properties<FKanbanTaskInput>> {
  return z.object({
    details_json: z.string().nullish(),
    state: z.string(),
    title: z.string()
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

export function FMarketplaceExpertInputSchema(): z.ZodObject<Properties<FMarketplaceExpertInput>> {
  return z.object({
    fexp_allow_tools: z.string(),
    fexp_app_capture_tools: z.string().default(""),
    fexp_block_tools: z.string(),
    fexp_name: z.string(),
    fexp_python_kernel: z.string(),
    fexp_system_prompt: z.string()
  })
}

export function FMcpServerInputSchema(): z.ZodObject<Properties<FMcpServerInput>> {
  return z.object({
    located_fgroup_id: z.string(),
    mcp_command: z.string(),
    mcp_description: z.string().default(""),
    mcp_enabled: z.boolean().default(false),
    mcp_env_vars: definedNonNullAnySchema.nullish(),
    mcp_name: z.string()
  })
}

export function FMcpServerPatchSchema(): z.ZodObject<Properties<FMcpServerPatch>> {
  return z.object({
    located_fgroup_id: z.string().nullish(),
    mcp_command: z.string().nullish(),
    mcp_description: z.string().nullish(),
    mcp_enabled: z.boolean().nullish(),
    mcp_env_vars: definedNonNullAnySchema.nullish(),
    mcp_name: z.string().nullish(),
    owner_shared: z.boolean().nullish()
  })
}

export function FPermissionPatchSchema(): z.ZodObject<Properties<FPermissionPatch>> {
  return z.object({
    perm_roles: z.number()
  })
}

export function FPersonaInputSchema(): z.ZodObject<Properties<FPersonaInput>> {
  return z.object({
    located_fgroup_id: z.string(),
    persona_discounts: z.string().nullish(),
    persona_marketable_name: z.string(),
    persona_marketable_version: z.string(),
    persona_name: z.string(),
    persona_setup: z.string()
  })
}

export function FPersonaPatchSchema(): z.ZodObject<Properties<FPersonaPatch>> {
  return z.object({
    located_fgroup_id: z.string().nullish(),
    persona_archived_ts: z.number().nullish(),
    persona_enabled: z.boolean().nullish(),
    persona_marketable_version: z.string().nullish(),
    persona_name: z.string().nullish(),
    persona_setup: z.string().nullish()
  })
}

export function FStatsAddInputSchema(): z.ZodObject<Properties<FStatsAddInput>> {
  return z.object({
    fgroup_id: z.string().default(""),
    st_chart: z.number(),
    st_how_many: definedNonNullAnySchema,
    st_involved_fexp_id: z.string().default(""),
    st_involved_fuser_id: z.string().default(""),
    st_involved_model: z.string().default(""),
    st_thing: z.string(),
    ws_id: z.string()
  })
}

export function FThreadInputSchema(): z.ZodObject<Properties<FThreadInput>> {
  return z.object({
    ft_app_capture: z.string().default(""),
    ft_app_searchable: z.string().default(""),
    ft_app_specific: z.string().default("null"),
    ft_error: z.string().default("null"),
    ft_fexp_id: z.string(),
    ft_persona_id: z.string().nullish(),
    ft_subchat_dest_ft_id: z.string().nullish(),
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
    ftm_usage: z.string().default("null").nullish(),
    ftm_user_preferences: z.string().default("null").nullish()
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
    ft_app_searchable: z.string().nullish(),
    ft_app_specific: z.string().nullish(),
    ft_archived_ts: z.number().nullish(),
    ft_confirmation_request: z.string().nullish(),
    ft_confirmation_response: z.string().nullish(),
    ft_error: z.string().nullish(),
    ft_subchat_dest_ft_id: z.string().nullish(),
    ft_title: z.string().nullish(),
    ft_toolset: z.string().nullish(),
    located_fgroup_id: z.string().nullish(),
    owner_shared: z.boolean().nullish()
  })
}

export function FUserProfilePatchSchema(): z.ZodObject<Properties<FUserProfilePatch>> {
  return z.object({
    fuser_experimental: z.boolean().nullish(),
    fuser_fullname: z.string().nullish()
  })
}

export function FWorkspaceCreateInputSchema(): z.ZodObject<Properties<FWorkspaceCreateInput>> {
  return z.object({
    ws_name: z.string()
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

export function RegisterInputSchema(): z.ZodObject<Properties<RegisterInput>> {
  return z.object({
    fullname: z.string(),
    password: z.string(),
    username: z.string()
  })
}
