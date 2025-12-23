/* eslint-disable */
import * as types from './graphql';
import type { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';

/**
 * Map of all GraphQL operations in the project.
 *
 * This map has several performance disadvantages:
 * 1. It is not tree-shakeable, so it will include all operations in the project.
 * 2. It is not minifiable, so the string of a GraphQL query will be multiple times inside the bundle.
 * 3. It does not support dead code elimination, so it will add unused operations.
 *
 * Therefore it is highly recommended to use the babel or swc plugin for production.
 * Learn more about it here: https://the-guild.dev/graphql/codegen/plugins/presets/preset-client#reducing-bundle-size
 */
type Documents = {
    "mutation CreateGroup($fgroup_name: String!, $fgroup_parent_id: String!) {\n  group_create(\n    input: {fgroup_name: $fgroup_name, fgroup_parent_id: $fgroup_parent_id}\n  ) {\n    fgroup_id\n    fgroup_name\n    ws_id\n    fgroup_parent_id\n    fgroup_created_ts\n  }\n}": typeof types.CreateGroupDocument,
    "subscription NavTreeSubs($ws_id: String!) {\n  tree_subscription(ws_id: $ws_id) {\n    treeupd_action\n    treeupd_id\n    treeupd_path\n    treeupd_type\n    treeupd_title\n  }\n}": typeof types.NavTreeSubsDocument,
    "query NavTreeWantWorkspaces {\n  query_basic_stuff {\n    fuser_id\n    workspaces {\n      ws_id\n      ws_owner_fuser_id\n      ws_root_group_id\n      root_group_name\n    }\n  }\n}": typeof types.NavTreeWantWorkspacesDocument,
};
const documents: Documents = {
    "mutation CreateGroup($fgroup_name: String!, $fgroup_parent_id: String!) {\n  group_create(\n    input: {fgroup_name: $fgroup_name, fgroup_parent_id: $fgroup_parent_id}\n  ) {\n    fgroup_id\n    fgroup_name\n    ws_id\n    fgroup_parent_id\n    fgroup_created_ts\n  }\n}": types.CreateGroupDocument,
    "subscription NavTreeSubs($ws_id: String!) {\n  tree_subscription(ws_id: $ws_id) {\n    treeupd_action\n    treeupd_id\n    treeupd_path\n    treeupd_type\n    treeupd_title\n  }\n}": types.NavTreeSubsDocument,
    "query NavTreeWantWorkspaces {\n  query_basic_stuff {\n    fuser_id\n    workspaces {\n      ws_id\n      ws_owner_fuser_id\n      ws_root_group_id\n      root_group_name\n    }\n  }\n}": types.NavTreeWantWorkspacesDocument,
};

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 *
 *
 * @example
 * ```ts
 * const query = graphql(`query GetUser($id: ID!) { user(id: $id) { name } }`);
 * ```
 *
 * The query argument is unknown!
 * Please regenerate the types.
 */
export function graphql(source: string): unknown;

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "mutation CreateGroup($fgroup_name: String!, $fgroup_parent_id: String!) {\n  group_create(\n    input: {fgroup_name: $fgroup_name, fgroup_parent_id: $fgroup_parent_id}\n  ) {\n    fgroup_id\n    fgroup_name\n    ws_id\n    fgroup_parent_id\n    fgroup_created_ts\n  }\n}"): (typeof documents)["mutation CreateGroup($fgroup_name: String!, $fgroup_parent_id: String!) {\n  group_create(\n    input: {fgroup_name: $fgroup_name, fgroup_parent_id: $fgroup_parent_id}\n  ) {\n    fgroup_id\n    fgroup_name\n    ws_id\n    fgroup_parent_id\n    fgroup_created_ts\n  }\n}"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "subscription NavTreeSubs($ws_id: String!) {\n  tree_subscription(ws_id: $ws_id) {\n    treeupd_action\n    treeupd_id\n    treeupd_path\n    treeupd_type\n    treeupd_title\n  }\n}"): (typeof documents)["subscription NavTreeSubs($ws_id: String!) {\n  tree_subscription(ws_id: $ws_id) {\n    treeupd_action\n    treeupd_id\n    treeupd_path\n    treeupd_type\n    treeupd_title\n  }\n}"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "query NavTreeWantWorkspaces {\n  query_basic_stuff {\n    fuser_id\n    workspaces {\n      ws_id\n      ws_owner_fuser_id\n      ws_root_group_id\n      root_group_name\n    }\n  }\n}"): (typeof documents)["query NavTreeWantWorkspaces {\n  query_basic_stuff {\n    fuser_id\n    workspaces {\n      ws_id\n      ws_owner_fuser_id\n      ws_root_group_id\n      root_group_name\n    }\n  }\n}"];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;