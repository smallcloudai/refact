import { useCallback, useEffect, useState } from "react";
import {
  DocumentationSettings,
  DocumentationSource,
} from "../components/DocumentationSettings";
import {
  DOCUMENTATION_ADD,
  DOCUMENTATION_LIST,
  DOCUMENTATION_REMOVE,
} from "../services/refact/consts";
import { useConfig } from "../hooks";

type DocListResponse = {
  url: string;
  max_depth: number;
  max_pages: number;
  pages: Record<string, string>;
};

function isArray(object: unknown): object is unknown[] {
  return Array.isArray(object);
}

function isDocListResponse(arr: unknown): arr is DocListResponse[] {
  if (!arr) return false;
  if (!isArray(arr)) return false;
  for (const x of arr) {
    if (!x) return false;
    if (typeof x !== "object") return false;
    if (!("url" in x)) return false;
    if (!("max_depth" in x)) return false;
    if (!("max_pages" in x)) return false;
    if (!("pages" in x)) return false;
    const { url, max_depth, max_pages, pages } = x;
    if (typeof url !== "string") return false;
    if (typeof max_depth !== "number") return false;
    if (typeof max_pages !== "number") return false;
    if (typeof pages !== "object") return false;
  }
  return true;
}

async function fetchDocumentationList(
  lspUrl: string | undefined,
): Promise<DocumentationSource[]> {
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

  if (!isDocListResponse(json)) {
    return [];
  }

  return json.map((x) => {
    return {
      url: x.url,
      maxDepth: x.max_depth,
      maxPages: x.max_pages,
      pages: Object.keys(x.pages).length,
    };
  });
}

export const Documentation: React.FC<{ goBack?: () => void }> = () => {
  const { lspUrl } = useConfig();
  const [documentationSources, setDocumentationSources] = useState<
    DocumentationSource[]
  >([]);

  const refetch = useCallback(async () => {
    const docSources = await fetchDocumentationList(lspUrl);
    setDocumentationSources(docSources);
  }, [lspUrl]);

  useEffect(() => {
    void refetch();
  }, [refetch]);

  const addDocumentation = (
    url: string,
    max_depth: number,
    max_pages: number,
  ) => {
    const f = async () => {
      const docsEndpoint = lspUrl
        ? `${lspUrl.replace(/\/*$/, "")}${DOCUMENTATION_ADD}`
        : DOCUMENTATION_ADD;

      const response = await fetch(docsEndpoint, {
        method: "POST",
        body: JSON.stringify({ source: url, max_depth, max_pages }),
      });

      if (!response.ok) {
        throw new Error(response.statusText);
      }

      await refetch();
    };
    void f();
  };

  const refetchDocumentation = (url: string) => {
    const document = documentationSources.find((value) => value.url === url);
    if (document !== undefined) {
      addDocumentation(url, document.maxDepth, document.maxPages);
    }
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

  return (
    <DocumentationSettings
      sources={documentationSources}
      addDocumentation={addDocumentation}
      deleteDocumentation={deleteDocumentation}
      editDocumentation={addDocumentation}
      refetchDocumentation={refetchDocumentation}
    />
  );
};
