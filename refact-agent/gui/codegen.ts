import type { CodegenConfig } from "@graphql-codegen/cli";

const config: CodegenConfig = {
  // schema: "https://app.refact.ai/v1/graphql", // requires flexus codebase to be running
  schema: "http://localhost:8008/v1/graphql",
  documents: ["src/**/*.(tsx|graphql)"],
  ignoreNoDocuments: true,
  generates: {
    "./generated/graphql/": {
      preset: "client",
      config: {
        useTypeImports: true,
      },
    },
    "./generated/documents.ts": {
      plugins: [
        "typescript",
        "typescript-operations",
        "typed-document-node",
        {
          "typescript-validation-schema": {
            schema: "zod",
          },
        },
      ],
      config: {
        useTypeImports: true,
      },
    },
    "./generated/schema.graphql": {
      plugins: ["schema-ast"],
      config: {
        includeDirectives: true,
      },
    },
  },
};

export default config;
