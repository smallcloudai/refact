import { RootState } from "../app/store";

export const getLspUrl = (state: RootState): string => {
  const port = state.config.lspPort;
  return `http://127.0.0.1:${port}`;
};