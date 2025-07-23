import { createSlice, PayloadAction } from "@reduxjs/toolkit/react";
import { graphqlQueriesAndMutations } from "../../services/graphql";
import {
  ExpertsForGroupQuery,
  ModelsForExpertQuery,
} from "../../../generated/documents";
import { setCurrentProjectInfo } from "../Chat/currentProject";

type InitialState = {
  selectedExpert:
    | ExpertsForGroupQuery["experts_effective_list"][number]["fexp_id"]
    | null;
  selectedModel:
    | ModelsForExpertQuery["expert_choice_consequences"]["models"][number]["provm_name"]
    | null;
};

const initialState: InitialState = {
  selectedExpert: null,
  selectedModel: null,
};

export const expertsSlice = createSlice({
  name: "experts",
  initialState,
  reducers: {
    setExpert: (state, action: PayloadAction<string>) => {
      state.selectedExpert = action.payload;
    },
    setModel: (state, action: PayloadAction<string>) => {
      state.selectedModel = action.payload;
    },
  },
  selectors: {
    selectCurrentExpert: (state) => state.selectedExpert,
    selectCurrentModel: (state) => state.selectedModel,
  },
  extraReducers(builder) {
    builder.addCase(setCurrentProjectInfo, () => {
      return initialState;
    });

    builder.addMatcher(
      graphqlQueriesAndMutations.endpoints.experts.matchFulfilled,
      (state, action) => {
        if (
          state.selectedExpert &&
          !action.payload.experts_effective_list.find(
            (expert) => state.selectedExpert === expert.fexp_id,
          )
        ) {
          state.selectedExpert = null;
        }
        if (
          !state.selectedExpert &&
          action.payload.experts_effective_list.length > 0
        ) {
          state.selectedExpert =
            action.payload.experts_effective_list[0].fexp_id;
        }
      },
    );

    builder.addMatcher(
      graphqlQueriesAndMutations.endpoints.modelsForExpert.matchFulfilled,
      (state, action) => {
        const names = action.payload.expert_choice_consequences.models.map(
          (model) => model.provm_name,
        );
        if (!state.selectedModel && names.length > 0) {
          state.selectedModel = names[0];
        }
        if (state.selectedModel && !names.includes(state.selectedModel)) {
          state.selectedModel = null;
        }
      },
    );

    // TODO: add case for restoring chat
  },
});

export const { selectCurrentExpert, selectCurrentModel } =
  expertsSlice.selectors;

export const { setExpert, setModel } = expertsSlice.actions;
