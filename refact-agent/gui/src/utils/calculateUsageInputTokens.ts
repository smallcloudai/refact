import { Usage } from "../services/refact";

export const calculateUsageInputTokens = (
  usage: Usage,
  keys: (keyof Usage)[],
): number =>
  keys.reduce((acc, key) => {
    const value = usage[key];
    return acc + (typeof value === "number" ? value : 0);
  }, 0);
