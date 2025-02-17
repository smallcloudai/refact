import { areToolParameters, type Integration } from "../../services/refact";
import { parseOrElse } from "../../utils";

export const convertRawIntegrationFormValues = (
  rawFormValues: Record<string, FormDataEntryValue>,
  schema: Integration["integr_schema"],
  currentIntegrationValues: Integration["integr_values"],
): NonNullable<Integration["integr_values"]> => {
  return Object.keys(rawFormValues).reduce<
    NonNullable<Integration["integr_values"]>
  >((acc, key) => {
    const field = schema.fields[key];
    const [f_type, _f_size] = (field.f_type as string).split("_");
    switch (f_type) {
      case "int":
        acc[key] = parseInt(rawFormValues[key] as string, 10);
        break;
      case "string":
        acc[key] = rawFormValues[key] as string;
        break;
      case "bool":
        acc[key] = rawFormValues[key] === "on" ? true : false;
        break;
      case "tool":
        acc[key] = parseOrElse<
          NonNullable<Integration["integr_values"]>[number]
        >(
          rawFormValues[key] as string,
          currentIntegrationValues &&
            areToolParameters(currentIntegrationValues.parameters)
            ? currentIntegrationValues.parameters
            : null,
        );
        break;
      case "output":
        acc[key] = parseOrElse<
          NonNullable<Integration["integr_values"]>[number]
        >(rawFormValues[key] as string, {});
        break;
      default:
        acc[key] = rawFormValues[key] as string;
        break;
    }
    return acc;
  }, {});
};
