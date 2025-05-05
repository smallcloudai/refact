declare module 'astro:content' {
	interface Render {
		'.mdx': Promise<{
			Content: import('astro').MarkdownInstance<{}>['Content'];
			headings: import('astro').MarkdownHeading[];
			remarkPluginFrontmatter: Record<string, any>;
		}>;
	}
}

declare module 'astro:content' {
	interface Render {
		'.md': Promise<{
			Content: import('astro').MarkdownInstance<{}>['Content'];
			headings: import('astro').MarkdownHeading[];
			remarkPluginFrontmatter: Record<string, any>;
		}>;
	}
}

declare module 'astro:content' {
	type Flatten<T> = T extends { [K: string]: infer U } ? U : never;

	export type CollectionKey = keyof AnyEntryMap;
	export type CollectionEntry<C extends CollectionKey> = Flatten<AnyEntryMap[C]>;

	export type ContentCollectionKey = keyof ContentEntryMap;
	export type DataCollectionKey = keyof DataEntryMap;

	type AllValuesOf<T> = T extends any ? T[keyof T] : never;
	type ValidContentEntrySlug<C extends keyof ContentEntryMap> = AllValuesOf<
		ContentEntryMap[C]
	>['slug'];

	export function getEntryBySlug<
		C extends keyof ContentEntryMap,
		E extends ValidContentEntrySlug<C> | (string & {}),
	>(
		collection: C,
		// Note that this has to accept a regular string too, for SSR
		entrySlug: E
	): E extends ValidContentEntrySlug<C>
		? Promise<CollectionEntry<C>>
		: Promise<CollectionEntry<C> | undefined>;

	export function getDataEntryById<C extends keyof DataEntryMap, E extends keyof DataEntryMap[C]>(
		collection: C,
		entryId: E
	): Promise<CollectionEntry<C>>;

	export function getCollection<C extends keyof AnyEntryMap, E extends CollectionEntry<C>>(
		collection: C,
		filter?: (entry: CollectionEntry<C>) => entry is E
	): Promise<E[]>;
	export function getCollection<C extends keyof AnyEntryMap>(
		collection: C,
		filter?: (entry: CollectionEntry<C>) => unknown
	): Promise<CollectionEntry<C>[]>;

	export function getEntry<
		C extends keyof ContentEntryMap,
		E extends ValidContentEntrySlug<C> | (string & {}),
	>(entry: {
		collection: C;
		slug: E;
	}): E extends ValidContentEntrySlug<C>
		? Promise<CollectionEntry<C>>
		: Promise<CollectionEntry<C> | undefined>;
	export function getEntry<
		C extends keyof DataEntryMap,
		E extends keyof DataEntryMap[C] | (string & {}),
	>(entry: {
		collection: C;
		id: E;
	}): E extends keyof DataEntryMap[C]
		? Promise<DataEntryMap[C][E]>
		: Promise<CollectionEntry<C> | undefined>;
	export function getEntry<
		C extends keyof ContentEntryMap,
		E extends ValidContentEntrySlug<C> | (string & {}),
	>(
		collection: C,
		slug: E
	): E extends ValidContentEntrySlug<C>
		? Promise<CollectionEntry<C>>
		: Promise<CollectionEntry<C> | undefined>;
	export function getEntry<
		C extends keyof DataEntryMap,
		E extends keyof DataEntryMap[C] | (string & {}),
	>(
		collection: C,
		id: E
	): E extends keyof DataEntryMap[C]
		? Promise<DataEntryMap[C][E]>
		: Promise<CollectionEntry<C> | undefined>;

	/** Resolve an array of entry references from the same collection */
	export function getEntries<C extends keyof ContentEntryMap>(
		entries: {
			collection: C;
			slug: ValidContentEntrySlug<C>;
		}[]
	): Promise<CollectionEntry<C>[]>;
	export function getEntries<C extends keyof DataEntryMap>(
		entries: {
			collection: C;
			id: keyof DataEntryMap[C];
		}[]
	): Promise<CollectionEntry<C>[]>;

	export function reference<C extends keyof AnyEntryMap>(
		collection: C
	): import('astro/zod').ZodEffects<
		import('astro/zod').ZodString,
		C extends keyof ContentEntryMap
			? {
					collection: C;
					slug: ValidContentEntrySlug<C>;
				}
			: {
					collection: C;
					id: keyof DataEntryMap[C];
				}
	>;
	// Allow generic `string` to avoid excessive type errors in the config
	// if `dev` is not running to update as you edit.
	// Invalid collection names will be caught at build time.
	export function reference<C extends string>(
		collection: C
	): import('astro/zod').ZodEffects<import('astro/zod').ZodString, never>;

	type ReturnTypeOrOriginal<T> = T extends (...args: any[]) => infer R ? R : T;
	type InferEntrySchema<C extends keyof AnyEntryMap> = import('astro/zod').infer<
		ReturnTypeOrOriginal<Required<ContentConfig['collections'][C]>['schema']>
	>;

	type ContentEntryMap = {
		"docs": {
"byok.md": {
	id: "byok.md";
  slug: "byok";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"contributing.md": {
	id: "contributing.md";
  slug: "contributing";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"faq.md": {
	id: "faq.md";
  slug: "faq";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/agent-integrations.md": {
	id: "features/agent-integrations.md";
  slug: "features/agent-integrations";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/ai-chat.md": {
	id: "features/ai-chat.md";
  slug: "features/ai-chat";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/ai-toolbox.md": {
	id: "features/ai-toolbox.md";
  slug: "features/ai-toolbox";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/ai-toolbox/comments.md": {
	id: "features/ai-toolbox/comments.md";
  slug: "features/ai-toolbox/comments";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/ai-toolbox/debug.md": {
	id: "features/ai-toolbox/debug.md";
  slug: "features/ai-toolbox/debug";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/ai-toolbox/explain-code.md": {
	id: "features/ai-toolbox/explain-code.md";
  slug: "features/ai-toolbox/explain-code";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/ai-toolbox/fix-bugs.md": {
	id: "features/ai-toolbox/fix-bugs.md";
  slug: "features/ai-toolbox/fix-bugs";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/ai-toolbox/improve-code.md": {
	id: "features/ai-toolbox/improve-code.md";
  slug: "features/ai-toolbox/improve-code";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/ai-toolbox/naming.md": {
	id: "features/ai-toolbox/naming.md";
  slug: "features/ai-toolbox/naming";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/getting-started.md": {
	id: "features/autonomous-agent/getting-started.md";
  slug: "features/autonomous-agent/getting-started";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/chrome.md": {
	id: "features/autonomous-agent/integrations/chrome.md";
  slug: "features/autonomous-agent/integrations/chrome";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/command-line-service.md": {
	id: "features/autonomous-agent/integrations/command-line-service.md";
  slug: "features/autonomous-agent/integrations/command-line-service";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/command-line-tool.md": {
	id: "features/autonomous-agent/integrations/command-line-tool.md";
  slug: "features/autonomous-agent/integrations/command-line-tool";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/docker.md": {
	id: "features/autonomous-agent/integrations/docker.md";
  slug: "features/autonomous-agent/integrations/docker";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/github.md": {
	id: "features/autonomous-agent/integrations/github.md";
  slug: "features/autonomous-agent/integrations/github";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/gitlab.md": {
	id: "features/autonomous-agent/integrations/gitlab.md";
  slug: "features/autonomous-agent/integrations/gitlab";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/index.md": {
	id: "features/autonomous-agent/integrations/index.md";
  slug: "features/autonomous-agent/integrations";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/mcp.md": {
	id: "features/autonomous-agent/integrations/mcp.md";
  slug: "features/autonomous-agent/integrations/mcp";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/mysql.md": {
	id: "features/autonomous-agent/integrations/mysql.md";
  slug: "features/autonomous-agent/integrations/mysql";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/pdb.md": {
	id: "features/autonomous-agent/integrations/pdb.md";
  slug: "features/autonomous-agent/integrations/pdb";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/postgresql.md": {
	id: "features/autonomous-agent/integrations/postgresql.md";
  slug: "features/autonomous-agent/integrations/postgresql";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/integrations/shell-commands.md": {
	id: "features/autonomous-agent/integrations/shell-commands.md";
  slug: "features/autonomous-agent/integrations/shell-commands";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/overview.md": {
	id: "features/autonomous-agent/overview.md";
  slug: "features/autonomous-agent/overview";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/rollback.md": {
	id: "features/autonomous-agent/rollback.md";
  slug: "features/autonomous-agent/rollback";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/autonomous-agent/tools.md": {
	id: "features/autonomous-agent/tools.md";
  slug: "features/autonomous-agent/tools";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/code-completion.md": {
	id: "features/code-completion.md";
  slug: "features/code-completion";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/context.md": {
	id: "features/context.md";
  slug: "features/context";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"features/finetuning.md": {
	id: "features/finetuning.md";
  slug: "features/finetuning";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/authentication/keycloak.md": {
	id: "guides/authentication/keycloak.md";
  slug: "guides/authentication/keycloak";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/deployment/aws/ec2.md": {
	id: "guides/deployment/aws/ec2.md";
  slug: "guides/deployment/aws/ec2";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/deployment/aws/getting-started.md": {
	id: "guides/deployment/aws/getting-started.md";
  slug: "guides/deployment/aws/getting-started";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/deployment/aws/marketplace.md": {
	id: "guides/deployment/aws/marketplace.md";
  slug: "guides/deployment/aws/marketplace";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/deployment/aws/usage.md": {
	id: "guides/deployment/aws/usage.md";
  slug: "guides/deployment/aws/usage";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/deployment/runpod.md": {
	id: "guides/deployment/runpod.md";
  slug: "guides/deployment/runpod";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/plugins/jetbrains/troubleshooting.md": {
	id: "guides/plugins/jetbrains/troubleshooting.md";
  slug: "guides/plugins/jetbrains/troubleshooting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/reverse-proxy.md": {
	id: "guides/reverse-proxy.md";
  slug: "guides/reverse-proxy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/usage-based-pricing.md": {
	id: "guides/usage-based-pricing.md";
  slug: "guides/usage-based-pricing";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/version-specific/enterprise/getting-started.md": {
	id: "guides/version-specific/enterprise/getting-started.md";
  slug: "guides/version-specific/enterprise/getting-started";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/version-specific/enterprise/license.md": {
	id: "guides/version-specific/enterprise/license.md";
  slug: "guides/version-specific/enterprise/license";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/version-specific/enterprise/model-hosting.md": {
	id: "guides/version-specific/enterprise/model-hosting.md";
  slug: "guides/version-specific/enterprise/model-hosting";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/version-specific/enterprise/plugins.md": {
	id: "guides/version-specific/enterprise/plugins.md";
  slug: "guides/version-specific/enterprise/plugins";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/version-specific/enterprise/users.md": {
	id: "guides/version-specific/enterprise/users.md";
  slug: "guides/version-specific/enterprise/users";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/version-specific/self-hosted.md": {
	id: "guides/version-specific/self-hosted.md";
  slug: "guides/version-specific/self-hosted";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"guides/version-specific/teams.md": {
	id: "guides/version-specific/teams.md";
  slug: "guides/version-specific/teams";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"index.mdx": {
	id: "index.mdx";
  slug: "index";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".mdx"] };
"installation/installation-hub.mdx": {
	id: "installation/installation-hub.mdx";
  slug: "installation/installation-hub";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".mdx"] };
"installation/jetbrains.md": {
	id: "installation/jetbrains.md";
  slug: "installation/jetbrains";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"installation/vs-code.md": {
	id: "installation/vs-code.md";
  slug: "installation/vs-code";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"introduction/quickstart.md": {
	id: "introduction/quickstart.md";
  slug: "introduction/quickstart";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"privacy.md": {
	id: "privacy.md";
  slug: "privacy";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"supported-models.md": {
	id: "supported-models.md";
  slug: "supported-models";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"supported-models/code-llama.md": {
	id: "supported-models/code-llama.md";
  slug: "supported-models/code-llama";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"supported-models/llama2.md": {
	id: "supported-models/llama2.md";
  slug: "supported-models/llama2";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"supported-models/refact-llm.md": {
	id: "supported-models/refact-llm.md";
  slug: "supported-models/refact-llm";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"supported-models/starcoder.md": {
	id: "supported-models/starcoder.md";
  slug: "supported-models/starcoder";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
"supported-models/wizard-coder.md": {
	id: "supported-models/wizard-coder.md";
  slug: "supported-models/wizard-coder";
  body: string;
  collection: "docs";
  data: InferEntrySchema<"docs">
} & { render(): Render[".md"] };
};

	};

	type DataEntryMap = {
		"i18n": {
};

	};

	type AnyEntryMap = ContentEntryMap & DataEntryMap;

	export type ContentConfig = typeof import("../src/content/config.js");
}
