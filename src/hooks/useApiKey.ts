import Cookies from "js-cookie";

export function useApiKey() {
  const maybeCookie = Cookies.get("api_key") ?? "";

  const setCookie = (value: string) => {
    Cookies.set("api_key", value);
  };

  return [maybeCookie, setCookie] as const;
}
