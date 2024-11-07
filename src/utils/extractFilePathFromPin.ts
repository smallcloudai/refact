export function extractFilePathFromPin(inputString: string): string {
  const start = inputString.indexOf('"') + 1; // Find the first quote and move one character forward
  const end = inputString.lastIndexOf('"'); // Find the last quote
  if (start !== end) {
    return inputString.substring(start, end); // Return the substring between the quotes
  }

  // fallback for old messages
  const [, , fileName] = inputString.split(" ");
  return fileName;
}
