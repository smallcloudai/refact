export const sizeValues = ["1", "2", "3", "4", "5", "6", "7", "8"] as const;
export const defaultSize = sizeValues[2];
export type AnimationSize = (typeof sizeValues)[number];
