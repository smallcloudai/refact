import type { Provider } from "../../../services/refact";

export type AggregatedProviderFields = {
  importantFields: Record<string, string | boolean>;
  extraFields: Record<string, string | boolean>;
};

const EXTRA_FIELDS_KEYS = [
  "embedding_endpoint",
  "completion_endpoint",
  "chat_endpoint",
];

export function aggregateProviderFields(providerData: Provider) {
  return Object.entries(providerData).reduce<AggregatedProviderFields>(
    (acc, [key, value]) => {
      const stringValue = value;

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
