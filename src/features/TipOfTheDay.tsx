import { createAction, createReducer } from "@reduxjs/toolkit";
import type { Config } from "../features/Config/configSlice";

type TipHost = "all" | "vscode";

function matchesHost(tipHost: TipHost, host: string): boolean {
  return tipHost === "all" || tipHost === host;
}

export const tips: [TipHost, string][] = [
  ["all", "Press 'Shift + Enter' to move to a new line in the Chat."],
  [
    "all",
    "Need a break from suggestions? You can pause them by clicking the 'Refact' icon in the status bar. The manual completion trigger still works: press [MANUAL_COMPLETION].",
  ],
  [
    "all",
    "Use the @file <file_name> command to attach a file to the chat context.",
  ],
  [
    "all",
    "To add the definition of a symbol to the chat context, use @definition <ClassOrFunctionName>.",
  ],
  [
    "vscode",
    "Need to find and fix bugs in your code? Select a piece of code, such as a function, press F1 to open the Toolbox, and write /bugs. It will also work on the whole file, provided it's not too large.",
  ],
  [
    "vscode",
    "Looking to edit your code? Select the lines you want to change, press F1, and write /edit <Instructions>.",
  ],
  [
    "vscode",
    "Need to explain code? Select the code snippet, press F1, and type /explain",
  ],
  [
    "vscode",
    "Need to summarize code? Select the part, press F1, and write /summarize",
  ],
  [
    "vscode",
    "Want to create new code? Ask Refact to do it for you: press F1 and write /gen with a task description.",
  ],
  [
    "vscode",
    "Make your code shorter: Select the code, press F1, and write /shorter",
  ],
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

export const next = createAction<Config>("tipOfTheDay/next");

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
    builder.addCase(next, (state, action) => {
      const keyBindings = action.payload.keyBindings;
      const host = action.payload.host;

      let tip: string | undefined = undefined;
      let next = state.next;

      while (tip === undefined) {
        const [tipHost, curTip] = tips[next];
        next = (next + 1) % tips.length;

        if (!matchesHost(tipHost, host)) {
          continue;
        }

        if (keyBindings?.completeManual !== undefined) {
          tip = curTip.replace(
            "[MANUAL_COMPLETION]",
            keyBindings.completeManual,
          );
        } else {
          tip = curTip.replace(
            "[MANUAL_COMPLETION]",
            "the key binding for manual completion",
          );
        }
      }

      return {
        next,
        tip,
      };
    });
  },
);
