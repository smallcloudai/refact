import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { chatAskQuestionThunk, chatResponse } from "../Chat";
import { isAssistantMessage, isDiffResponse } from "../../events";
import { parseOrElse, partition } from "../../utils";

export type PatchMeta = {
  chatId: string;
  toolCallId: string;
  filePath: string;
  started: boolean;
  completed: boolean;
};

const initialState: { patches: PatchMeta[] } = { patches: [] };

export const patchesAndDiffsTrackerSlice = createSlice({
  name: "patchesAndDiffsTracker",
  initialState,
  reducers: {
    addPatchMeta: (state, action: PayloadAction<PatchMeta>) => {
      state.patches.push(action.payload);
    },

    removePatchMetaByFileNameIfCompleted: (
      state,
      action: PayloadAction<string[]>,
    ) => {
      const next = state.patches.filter((patchMeta) => {
        if (!patchMeta.completed) return true;
        return !action.payload.includes(patchMeta.filePath);
      });
      state.patches = next;
    },

    setStartedByFilePaths: (state, action: PayloadAction<string[]>) => {
      const next = state.patches.map((patchMeta) => {
        if (action.payload.includes(patchMeta.filePath)) {
          return { ...patchMeta, started: true };
        } else {
          return patchMeta;
        }
      });
      state.patches = next;
    },
  },

  extraReducers: (builder) => {
    builder.addCase(chatAskQuestionThunk.pending, (state, action) => {
      if (!action.meta.arg.toolsConfirmed) return state;
      if (action.meta.arg.messages.length === 0) return state;
      const { messages, chatId } = action.meta.arg;
      const lastMessage = messages[messages.length - 1];
      if (!isAssistantMessage(lastMessage)) return state;
      const toolCalls = lastMessage.tool_calls;
      if (!toolCalls) return state;
      const patches = toolCalls.reduce<PatchMeta[]>((acc, toolCall) => {
        if (toolCall.id === undefined) return acc;
        if (toolCall.function.name !== "patch") return acc;
        const filePath = pathFromArgString(toolCall.function.arguments);
        if (!filePath) return acc;
        return [
          ...acc,
          {
            chatId,
            toolCallId: toolCall.id,
            filePath,
            started: false,
            completed: false,
          },
        ];
      }, []);
      state.patches.push(...patches);
    });

    builder.addCase(chatResponse, (state, action) => {
      if (!isDiffResponse(action.payload)) return state;
      const { id, tool_call_id } = action.payload;
      const next = state.patches.map((patchMeta) => {
        if (patchMeta.chatId !== id) return patchMeta;
        if (patchMeta.toolCallId !== tool_call_id) return patchMeta;
        return { ...patchMeta, completed: true };
      });

      state.patches = next;
    });
  },

  selectors: {
    selectUnsentPatchesFilePaths: (state) => {
      const [unstarted, started] = partition(
        state.patches,
        (patchMeta) => patchMeta.started,
      );
      const unstaredFilePaths = unstarted.map(
        (patchMeta) => patchMeta.filePath,
      );
      const startedFilePaths = started.map((patchMeta) => patchMeta.filePath);
      return unstaredFilePaths.filter(
        (filePath) => !startedFilePaths.includes(filePath),
      );
    },
    selectCompletedPatchesFilePaths: (state) => {
      const [incomplete, completed] = partition(
        state.patches,
        (patchMeta) => patchMeta.completed,
      );
      const incompleteFilePaths = incomplete.map(
        (patchMeta) => patchMeta.filePath,
      );
      const completeFilePaths = completed.map(
        (patchMeta) => patchMeta.filePath,
      );
      return completeFilePaths.filter(
        (filePath) => !incompleteFilePaths.includes(filePath),
      );
    },

    selectAllFilePaths: (state) => {
      return state.patches.map((patchMeta) => patchMeta.filePath);
    },
  },
});

export const {
  selectCompletedPatchesFilePaths,
  selectUnsentPatchesFilePaths,
  selectAllFilePaths,
} = patchesAndDiffsTrackerSlice.selectors;

export const { setStartedByFilePaths, removePatchMetaByFileNameIfCompleted } =
  patchesAndDiffsTrackerSlice.actions;

const pathFromArgString = (argString: string) => {
  const args = parseOrElse<Record<string, unknown> | null>(argString, null);
  if (
    args &&
    typeof args === "object" &&
    "path" in args &&
    typeof args.path === "string"
  ) {
    return args.path;
  } else {
    return null;
  }
};
