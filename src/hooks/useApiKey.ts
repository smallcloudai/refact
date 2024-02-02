import * as ApiKey from "../utils/ApiKey";

export function useApiKey(): [string, (value: string) => void] {
  const maybeCookie = ApiKey.getApiKey();

  const setCookie = (value: string) => {
    ApiKey.setApiKey(value);
  };

  return [maybeCookie, setCookie];
}
