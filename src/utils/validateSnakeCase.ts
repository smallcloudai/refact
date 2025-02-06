export const validateSnakeCase = (value: string) => {
  const snakeCaseRegex = /^^[a-zA-Z0-9_-]{1,64}$/;
  return snakeCaseRegex.test(value);
};
