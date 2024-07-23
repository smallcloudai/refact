import { useCallback, useEffect, useState } from "react";
import { useConfig } from "../contexts/config-context";
import {
  DocumentationSettings,
  DocumentationSource,
} from "../components/DocumentationSettings";
import {
  DOCUMENTATION_ADD,
  DOCUMENTATION_LIST,
  DOCUMENTATION_REMOVE,
} from "../services/refact/consts";

async function fetchDocumentationList(
  lspUrl: string | undefined,
): Promise<string[]> {
  const docsEndpoint = lspUrl
    ? `${lspUrl.replace(/\/*$/, "")}${DOCUMENTATION_LIST}`
    : DOCUMENTATION_LIST;

  const response = await fetch(docsEndpoint, {
    method: "GET",
    credentials: "same-origin",
    headers: {
      accept: "application/json",
    },
  });

  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const json: unknown = await response.json();

  if (!Array.isArray(json)) {
    return [];
  }

  return json as string[];
}

export const Documentation: React.FC<{ goBack?: () => void }> = () => {
  const { lspUrl } = useConfig();
  const [documentationSources, setDocumentationSources] = useState<
    DocumentationSource[]
  >([]);

  const refetch = useCallback(async () => {
    const docSources = await fetchDocumentationList(lspUrl);
    setDocumentationSources(
      docSources.map((source) => {
        return {
          url: source,
          maxDepth: 2,
          maxPages: 50,
          pages: 1,
        };
      }),
    );
  }, [lspUrl]);

  useEffect(() => {
    void refetch();
  }, [refetch]);

  const addDocumentation = (url: string) => {
    const f = async () => {
      const docsEndpoint = lspUrl
        ? `${lspUrl.replace(/\/*$/, "")}${DOCUMENTATION_ADD}`
        : DOCUMENTATION_ADD;

      const response = await fetch(docsEndpoint, {
        method: "POST",
        body: JSON.stringify({ source: url }),
      });

      if (!response.ok) {
        throw new Error(response.statusText);
      }

      await refetch();
    };
    void f();
  };
  const deleteDocumentation = (url: string) => {
    const f = async () => {
      const docsEndpoint = lspUrl
        ? `${lspUrl.replace(/\/*$/, "")}${DOCUMENTATION_REMOVE}`
        : DOCUMENTATION_REMOVE;

      const response = await fetch(docsEndpoint, {
        method: "POST",
        body: JSON.stringify({ source: url }),
      });

      if (!response.ok) {
        throw new Error(response.statusText);
      }

      await refetch();
    };
    void f();
  };
  const editDocumentation = (
    _url: string,
    _maxDepth: number,
    _maxPages: number,
  ) => {
    return 0;
  };

  return (
    <DocumentationSettings
      sources={documentationSources}
      addDocumentation={addDocumentation}
      deleteDocumentation={deleteDocumentation}
      editDocumentation={editDocumentation}
    />
  );
};
