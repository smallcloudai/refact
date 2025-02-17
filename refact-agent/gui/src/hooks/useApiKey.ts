import { useState, useEffect } from "react";
import * as ApiKey from "../utils/ApiKey";

export function useApiKey(): [string, (value: string) => void] {
  const maybeKey = ApiKey.getApiKey();
  const [key, setKey] = useState(maybeKey);
  useEffect(() => {
    const apiKey = ApiKey.getApiKey();
    setKey(apiKey);
  }, []);

  const setApiKey = (value: string) => {
    ApiKey.setApiKey(value);
    setKey(value);
  };

  return [key, setApiKey];
}
