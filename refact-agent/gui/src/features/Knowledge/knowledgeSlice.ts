import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import type { MemoRecord, VecDbStatus } from "../../services/refact/types";

export type KnowledgeState = {
  loaded: boolean;
  memories: Record<string, MemoRecord>;
  status: null | VecDbStatus;
};

const initialState: KnowledgeState = {
  loaded: false,
  memories: {},
  status: null,
};

export const knowledgeSlice = createSlice({
  name: "knowledge",
  initialState,
  reducers: {
    // TODO: add reducers
    setVecDbStatus: (state, action: PayloadAction<VecDbStatus>) => {
      state.loaded = true;
      state.status = action.payload;
    },
    setMemory: (state, action: PayloadAction<MemoRecord>) => {
      state.loaded = true;
      state.memories[action.payload.memid] = action.payload;
    },
    deleteMemory: (state, action: PayloadAction<string>) => {
      state.loaded = true;
      const { [action.payload]: _, ...memories } = state.memories;
      state.memories = memories;
    },
    clearMemory: (state) => {
      state.loaded = true;
      state.memories = {};
    },
  },
  // TODO: selectors
  selectors: {
    selectVecDbStatus: (state) => state.status,
    selectMemories: (state) => state.memories,
    selectKnowledgeIsLoaded: (state) => state.loaded,
  },
});

export const { setVecDbStatus, setMemory, deleteMemory, clearMemory } =
  knowledgeSlice.actions;

export const { selectVecDbStatus, selectMemories, selectKnowledgeIsLoaded } =
  knowledgeSlice.selectors;
