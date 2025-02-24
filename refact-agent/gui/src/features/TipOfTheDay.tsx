import { createSlice } from "@reduxjs/toolkit";
import type { Config } from "../features/Config/configSlice";

type TipHost = "all" | "vscode";

function matchesHost(tipHost: TipHost, host: string): boolean {
  return tipHost === "all" || tipHost === host;
}

export const tips: [TipHost, string][] = [
  ["all", "Press 'Shift + Enter' to move to a new line in the chat input box."],
  [
    "all",
    "Need a break from code suggestions? You can pause them by clicking the 'Refact' icon in the status bar. On pause, the manual code completion still works: press [MANUAL_COMPLETION].",
  ],
  ["all", "Use @file <file_name> to attach a file to the chat context."],
  [
    "all",
    "Use @definition <class_or_function_name> to add the definition to the chat context. You can even combine it with text, for example: what is the relationship between @definition Frog and @definition Toad?",
  ],
  [
    "all",
    "After you did something useful with one piece of code, you can select another piece of code and ask the model to do the same thing as a follow-up.",
  ],
  [
    "all",
    "The quickest way to call chat is to press F1. You can change key bindings in the menu.",
  ],
  [
    "all",
    'Before hitting F1, you can select a few lines of code and the chat will automatically tick the "Attach" checkboxes. After you get a modified version of the code from chat, you can paste it back, IDE will show diff for you to see the changes.',
  ],
  [
    "all",
    'You can manually combine @file @definition @references @search to collect a lot of context, and ask a complex question. Try it, the chat handles complex questions pretty well! If there is too much code to fit into model limits, context post-processing will start to skeletonize the code -- replace function bodies with "..."',
  ],
  [
    "all",
    "You can rely on exploration tools to collect the context for you. For this to work better, write class and function names exactly, or ask the model to search for a specific thing first.",
  ],
  [
    "all",
    "Use @web http://link-to-somewhere/ to add a web page to the context. Or just give model the URL it will use the web() exploration tool to check it out.",
  ],
];

export type TipOfTheDayState = {
  current: number;
  tip: string;
};

const initialState: TipOfTheDayState = {
  current: 0,
  tip: "", //tips[0][1], // make sure can be all
};

export const tipOfTheDaySlice = createSlice({
  name: "tipOfTheDay",
  initialState,
  reducers: (create) => ({
    nextTip: create.reducer<{ host: Config["host"]; completeManual?: string }>(
      (state, action) => {
        const { host, completeManual } = action.payload;

        let tip: string | undefined = undefined;
        let nextIndex = (state.current + 1) % tips.length;

        while (tip === undefined) {
          const [tipHost, curTip] = tips[nextIndex];
          nextIndex = (nextIndex + 1) % tips.length;

          if (!matchesHost(tipHost, host)) {
            continue;
          }

          if (completeManual !== undefined) {
            tip = curTip.replace("[MANUAL_COMPLETION]", completeManual);
          } else {
            tip = curTip.replace(
              "[MANUAL_COMPLETION]",
              "the key binding for manual completion",
            );
          }
        }

        return {
          current: nextIndex,
          tip,
        };
      },
    ),
  }),
  selectors: {
    currentTipOfTheDay: (state) => state.tip,
  },
});

export const { nextTip } = tipOfTheDaySlice.actions;
export const { currentTipOfTheDay } = tipOfTheDaySlice.selectors;
