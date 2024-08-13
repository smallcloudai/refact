import { createAction, createReducer } from "@reduxjs/toolkit";

// todo: get shortcuts from settings
export const tips: string[] = [
  "Press 'Shift + Enter' to move to a new line in the Chat.",
  "Need a break from suggestions? You can pause them by clicking the 'Refact' icon in the status bar. The manual completion trigger still works: press 'Option + Space'.",
  "Use the @file <file_name> command to attach a file to the chat context. To add the definition of a symbol to the chat context, use @definition <ClassOrFunctionName>.",
  "Need to find and fix bugs in your code? Select a piece of code, such as a function, press F1 to open the Toolbox, and write /bugs. It will also work on the whole file, provided it's not too large.",
  "Looking to edit your code? Select the lines you want to change, press F1, and write /edit <Instructions>.",
  "Need to explain code? Select the code snippet, press F1, and type /explain",
  "Need to summarize code? Select the part, press F1, and write /summarize",
  "Want to create new code? Ask Refact to do it for you: press F1 and write /gen with a task description.",
  "Make your code shorter: Select the code, press F1, and write /shorter",
];

export type TipOfTheDayState = {
  next: number;
  tip: string;
};

function isTipOfTheDayState(state: unknown): state is TipOfTheDayState {
  if (!state) return false;
  if (typeof state !== "object") return false;
  if (!("next" in state)) return false;
  if (typeof state.next !== "number") return false;
  if (!("tip" in state)) return false;
  if (typeof state.tip !== "string") return false;
  return true;
}

const initialState: TipOfTheDayState = {
  next: 0,
  tip: "",
};

export const next = createAction("tipOfTheDay/next");

function loadFromLocalStorage(): TipOfTheDayState {
  try {
    const serialisedState = localStorage.getItem("tipOfTheDay");
    if (serialisedState === null) return initialState;
    const parsedState: unknown = JSON.parse(serialisedState);
    if (!isTipOfTheDayState(parsedState)) return initialState;
    return parsedState;
  } catch (e) {
    // eslint-disable-next-line no-console
    console.warn(e);
    return initialState;
  }
}

export const saveTipOfTheDayToLocalStorage = (state: {
  tipOfTheDay: TipOfTheDayState;
}) => {
  try {
    localStorage.setItem("tipOfTheDay", JSON.stringify(state.tipOfTheDay));
  } catch (e) {
    // eslint-disable-next-line no-console
    console.warn(e);
  }
};

export const tipOfTheDayReducer = createReducer<TipOfTheDayState>(
  loadFromLocalStorage(),
  (builder) => {
    builder.addCase(next, (state) => {
      return {
        next: (state.next + 1) % tips.length,
        tip: tips[state.next % tips.length],
      };
    });
  },
);
