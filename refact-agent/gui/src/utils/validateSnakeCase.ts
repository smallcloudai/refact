export const validateSnakeCase = (value: string) => {
  // Check length constraints
  if (value.length === 0 || value.length > 64) {
    return false;
  }

  // Proper snake_case regex:
  // - Must start with lowercase letter
  // - Can contain lowercase letters, digits, and underscores
  // - Underscores must be followed by at least one alphanumeric character
  // - No consecutive underscores, no trailing underscores
  const snakeCaseRegex = /^[a-z][a-z0-9]*(?:_[a-z0-9]+)*$/;
  return snakeCaseRegex.test(value);
};
