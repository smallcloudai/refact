import { ToolCall } from "../../events";

// export const TAKE_NOTE_MESSAGE = [
//   'How many times user has corrected or directed you? Write "Number of correction points N".',
//   'Then start each one with "---\n", describe what you (the assistant) did wrong, write "Mistake: ..."',
//   'Write documentation to tools or the project in general that will help you next time, describe in detail how tools work, or what the project consists of, write "Documentation: ..."',
//   "A good documentation for a tool describes what is it for, how it helps to answer user's question, what applicability criteia were discovered, what parameters work and how it will help the user.",
//   "A good documentation for a project describes what folders, files are there, summarization of each file, classes. Start documentation for the project with project name.",
//   "After describing all points, call note_to_self() in parallel for each actionable point, generate keywords that should include the relevant tools, specific files, dirs, and put documentation-like paragraphs into text.",
// ].join("\n");

// export const TAKE_NOTE_MESSAGE = [
//   "How many times user has corrected you about tool usage? Call note_to_self() with this exact format:",
//   "",
//   "CORRECTION_POINTS: N",
//   "",
//   "POINT1 WHAT_I_DID_WRONG: i should have used ... tool call or method or plan ... instead of this tool call or method or plan",
//   "POINT1 WAS_I_SUCCESSFUL_AFTER_CORRECTION: YES/NO",
//   "POINT1 FOR_FUTURE_FEREFENCE: when ... [describe situation when it's applicable] use ... tool call or method or plan",
//   "POINT1 HOW_NEW_IS_THIS_NOTE: 0-5",
//   "POINT1 HOW_INSIGHTFUL_IS_THIS_NOTE: 0-5",
//   "",
//   "POINT2 WHAT_I_DID_WRONG: ...",
//   "POINT2 WAS_I_SUCCESSFUL_AFTER_CORRECTION: ...",
//   "POINT2 FOR_FUTURE_FEREFENCE: ...",
//   "POINT2 HOW_NEW_IS_THIS_NOTE: ...",
//   "POINT2 HOW_INSIGHTFUL_IS_THIS_NOTE: ...",
// ].join("\n");

export const TAKE_NOTE_MESSAGE = `How many times did you used a tool incorrectly, so it didn't produce the indended result? Call remember_how_to_use_tools() with this exact format:

CORRECTION_POINTS: N

POINT1 WHAT_I_DID_WRONG: i should have used ... tool call or method or plan ... instead of this tool call or method or plan.
POINT1 FOR_FUTURE_FEREFENCE: when ... [describe situation when it's applicable] use ... tool call or method or plan.

POINT2 WHAT_I_DID_WRONG: ...
POINT2 FOR_FUTURE_FEREFENCE: ...
`;

function mergeToolCall(prev: ToolCall[], add: ToolCall): ToolCall[] {
  const calls = prev.slice();

  if (calls[add.index]) {
    const prevCall = calls[add.index];
    const prevArgs = prevCall.function.arguments;
    const nextArgs = prevArgs + add.function.arguments;
    const call: ToolCall = {
      ...prevCall,
      function: {
        ...prevCall.function,
        arguments: nextArgs,
      },
    };
    calls[add.index] = call;
  } else {
    calls[add.index] = add;
  }
  return calls;
}

export function mergeToolCalls(prev: ToolCall[], add: ToolCall[]): ToolCall[] {
  return add.reduce((acc, cur) => {
    return mergeToolCall(acc, cur);
  }, prev);
}
