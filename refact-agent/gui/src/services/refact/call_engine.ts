import { RootState } from "../../app/store";

export interface CallEngineConfig {
  method?: string;
  credentials?: RequestCredentials;
  redirect?: RequestRedirect;
  cache?: RequestCache;
  headers?: HeadersInit;
  body?: BodyInit | null;
  signal?: AbortSignal;
  stream?: boolean;
}

export function getServerUrl(state: RootState, endpoint: string): string {
  const port = state.config.lspPort;
  let host = state.config.lspUrl || 'http://127.0.0.1';
  if (port) {
    //replace port in host
    const url = new URL(host);
    url.port = port.toString();
    host = url.toString();
  }

  if (endpoint.startsWith('/')) {
    endpoint = endpoint.substring(1);
  }

  console.log("URL", `${host}${endpoint}`);

  return `${host}${endpoint}`;
}

export async function callEngine<T>(
  state: RootState,
  endpoint: string,
  config: CallEngineConfig = {}
): Promise<T> {
  const url = getServerUrl(state, endpoint);
  
  const defaultConfig: RequestInit = {
    method: config.method ?? 'GET',
    mode: 'cors',  // Enable CORS mode
    redirect: config.redirect ?? 'follow',
    cache: config.cache ?? 'no-cache',
    headers: {
      'Accept': 'application/json',
      'Content-Type': 'application/json',
      ...config.headers
    },
    body: config.body,
    signal: config.signal
  };

  if (state.config.apiKey) {
    (defaultConfig.headers as Record<string, string>)['Authorization'] = `Bearer ${state.config.apiKey}`;
  }

  console.log("callEngine", defaultConfig);

  const response = await fetch(url, defaultConfig);
  
  if (!response.ok) {
    throw new Error(response.statusText);
  }

  const result = await response.json();
  console.log("result", result);
  
  return result as T;  // Fixed: Don't call response.json() twice
}

export async function pollEngine<T>(
  state: RootState,
  endpoint: string,
  config: CallEngineConfig = {},
  pollInterval: number = 1000
): Promise<T> {
  const url = getServerUrl(state, endpoint);
  
  return new Promise((resolve, reject) => {
    const poll = async () => {
      try {
        const defaultConfig: RequestInit = {
          method: config.method ?? 'GET',
          credentials: config.credentials ?? 'same-origin',
          redirect: config.redirect ?? 'follow',
          cache: config.cache ?? 'no-cache',
          headers: config.headers ?? {},
    body: config.body,
    signal: config.signal
        };

        if (state.config.apiKey) {
          (defaultConfig.headers as Record<string, string>)['Authorization'] = `Bearer ${state.config.apiKey}`;
        }

        const response = await fetch(url, defaultConfig);
        
        if (response.ok) {
          const data = await response.text();
          resolve(data as T);
        } else {
          throw new Error(response.statusText);
        }
      } catch (err) {
        if (err instanceof Error && err.message === "Failed to fetch") {
          setTimeout(poll, pollInterval);
        } else {
          reject(err);
        }
      }
    };
    poll();
  });
}