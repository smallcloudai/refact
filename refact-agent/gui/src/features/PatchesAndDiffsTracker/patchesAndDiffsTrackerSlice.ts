import { createSelector, createSlice, PayloadAction } from "@reduxjs/toolkit";
import { applyChatEvent } from "../Chat/Thread/actions";
import { partition } from "../../utils";
import { RootState } from "../../app/store";
import { isDiffMessage } from "../../services/refact";

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
    // Listen to SSE events for diff messages
    builder.addCase(applyChatEvent, (state, action) => {
      const { chat_id, ...event } = action.payload;
      // Check for message_added events with diff role
      if (event.type === "message_added") {
        const msg = event.message;
        if (isDiffMessage(msg)) {
          const tool_call_id = "tool_call_id" in msg ? msg.tool_call_id : undefined;
          if (tool_call_id) {
            const next = state.patches.map((patchMeta) => {
              if (patchMeta.chatId !== chat_id) return patchMeta;
              if (patchMeta.toolCallId !== tool_call_id) return patchMeta;
              return { ...patchMeta, completed: true };
            });
            state.patches = next;
          }
        }
      }
    });
  },

  selectors: {
    selectAllFilePaths: (state) => {
      return state.patches.map((patchMeta) => patchMeta.filePath);
    },
  },
});

export const { selectAllFilePaths } = patchesAndDiffsTrackerSlice.selectors;

export const selectUnsentPatchesFilePaths = createSelector(
  [(state: RootState) => state.patchesAndDiffsTracker],
  (state) => {
    const [unstarted, started] = partition(
      state.patches,
      (patchMeta) => patchMeta.started,
    );
    const unstartedFilePaths = unstarted.map((patchMeta) => patchMeta.filePath);
    const startedFilePaths = started.map((patchMeta) => patchMeta.filePath);
    return unstartedFilePaths.filter(
      (filePath) => !startedFilePaths.includes(filePath),
    );
  },
);

export const selectCompletedPatchesFilePaths = createSelector(
  [(state: RootState) => state.patchesAndDiffsTracker],
  (state) => {
    const [incomplete, completed] = partition(
      state.patches,
      (patchMeta) => patchMeta.completed,
    );
    const incompleteFilePaths = incomplete.map(
      (patchMeta) => patchMeta.filePath,
    );
    const completeFilePaths = completed.map((patchMeta) => patchMeta.filePath);
    return completeFilePaths.filter(
      (filePath) => !incompleteFilePaths.includes(filePath),
    );
  },
);

export const { setStartedByFilePaths, removePatchMetaByFileNameIfCompleted } =
  patchesAndDiffsTrackerSlice.actions;
