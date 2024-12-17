export const validateSnakeCase = (value: string) => {
  const snakeCaseRegex = /^[a-z0-9]+(_[a-z0-9]+)*$/;
  return snakeCaseRegex.test(value);
};
