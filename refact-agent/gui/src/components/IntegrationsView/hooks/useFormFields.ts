import { useState, useMemo } from "react";
import type {
  IntegrationField,
  IntegrationPrimitive,
} from "../../../services/refact";

export const useFormFields = (
  fields?: Record<string, IntegrationField<NonNullable<IntegrationPrimitive>>>,
) => {
  const [areExtraFieldsRevealed, setAreExtraFieldsRevealed] = useState(false);

  const { importantFields, extraFields } = useMemo(() => {
    if (!fields) {
      return { importantFields: {}, extraFields: {} };
    }

    return Object.entries(fields).reduce(
      (acc, [key, field]) => {
        if (field.f_extra) {
          acc.extraFields[key] = field;
        } else {
          acc.importantFields[key] = field;
        }
        return acc;
      },
      {
        importantFields: {} as Record<
          string,
          IntegrationField<NonNullable<IntegrationPrimitive>>
        >,
        extraFields: {} as Record<
          string,
          IntegrationField<NonNullable<IntegrationPrimitive>>
        >,
      },
    );
  }, [fields]);

  const toggleExtraFields = () => setAreExtraFieldsRevealed((prev) => !prev);

  return {
    importantFields,
    extraFields,
    areExtraFieldsRevealed,
    toggleExtraFields,
  };
};
