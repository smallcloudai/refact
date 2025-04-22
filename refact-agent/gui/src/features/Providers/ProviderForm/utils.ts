import type { Provider } from "../../../services/refact";

export type AggregatedProviderFields = {
  importantFields: Record<string, string | boolean>;
  extraFields: Record<string, string | boolean>;
};

const EXTRA_FIELDS_KEYS = [
  "embedding_endpoint",
  "completion_endpoint",
  "chat_endpoint",
  "tokenizer_api_key",
];
const HIDDEN_FIELDS_KEYS = [
  "name",
  "readonly",
  "enabled",
  "supports_completion",
];

export function aggregateProviderFields(providerData: Provider) {
  return Object.entries(providerData).reduce<AggregatedProviderFields>(
    (acc, [key, value]) => {
      const stringValue = value;

      if (HIDDEN_FIELDS_KEYS.some((hiddenField) => hiddenField === key)) {
        return acc;
      }

      if (EXTRA_FIELDS_KEYS.some((extraField) => extraField === key)) {
        acc.extraFields[key] = stringValue;
      } else {
        acc.importantFields[key] = stringValue;
      }

      return acc;
    },
    { importantFields: {}, extraFields: {} },
  );
}
