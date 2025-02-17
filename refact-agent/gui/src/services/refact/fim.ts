type FimChoices = {
  code_completion: string;
  finish_reason: string;
  index: number;
}[];

type FimFile = {
  file_content: string;
  file_name: string;
  line1: number;
  line2: number;
};

type ContextFiles = FimFile[];

export type ContextBucket = {
  file_path: string;
  line1: number;
  line2: number;
  name: string;
};

export type Buckets = ContextBucket[];

export type FIMContext = {
  attached_files?: ContextFiles;

  bucket_declarations?: Buckets;
  bucket_usage_of_same_stuff?: Buckets;
  bucket_high_overlap?: Buckets;
  cursor_symbols?: Buckets;

  fim_ms?: number;
  n_ctx?: number;
  rag_ms?: number;
  rag_tokens_limit?: number;
};

export type FimDebugData = {
  choices: FimChoices;
  snippet_telemetry_id: number;
  model: string;
  context?: FIMContext;
  created?: number;
  elapsed?: number;
  cached?: boolean;
};
