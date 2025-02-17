import Cookies from "js-cookie";
export const getApiKey = () => Cookies.get("api_key") ?? "";

export const setApiKey = (value: string) => {
  Cookies.set("api_key", value);
};
