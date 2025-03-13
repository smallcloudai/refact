import { Usage } from "../services/refact";

export const calculateUsageInputTokens = ({
  keys,
  usage,
}: {
  keys: (keyof Usage)[];
  usage?: Usage;
}): number => {
  return keys.reduce((acc, key) => {
    if (!(usage && key in usage)) return acc;
    const value = usage[key];
    return acc + (typeof value === "number" ? value : 0);
  }, 0);
};
