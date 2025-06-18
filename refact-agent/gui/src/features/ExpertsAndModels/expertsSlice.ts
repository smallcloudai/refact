import { createSlice, PayloadAction } from "@reduxjs/toolkit/react";
import {
  getExpertsThunk,
  getModelsForExpertThunk,
} from "../../services/graphql/graphqlThunks";
import {
  ExpertsForGroupQuery,
  ModelsForExpertQuery,
} from "../../../generated/documents";
import { setCurrentProjectInfo } from "../Chat/currentProject";

type InitialState = {
  loading: boolean;
  error: string | null;
  experts: ExpertsForGroupQuery["experts_effective_list"];
  selectedExpert:
    | ExpertsForGroupQuery["experts_effective_list"][number]["fexp_id"]
    | null;
  modelsForExperts: {
    loading: boolean;
    models: Record<
      ExpertsForGroupQuery["experts_effective_list"][number]["fexp_id"],
      ModelsForExpertQuery["expert_choice_consequences"]
    >;
  };
  selectedModel:
    | ModelsForExpertQuery["expert_choice_consequences"][number]["provm_name"]
    | null;
};

const initialState: InitialState = {
  // probally don't need workspace
  // workspace: null,
  loading: false,
  error: null,
  experts: [],
  selectedExpert: null,
  modelsForExperts: {
    loading: false,
    models: {},
  },
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
    selectAvailableExperts: (state) => state.experts,
    selectIsExpertsLoading: (state) => state.loading,
    selectCurrentModel: (state) => state.selectedModel,
    selectModelsForExpert: (state) => {
      if (!state.selectedExpert) return [];
      if (!(state.selectedExpert in state.modelsForExperts.models)) return [];
      return state.modelsForExperts.models[state.selectedExpert];
    },
    selectModelsForExpertLoading: (state) => state.modelsForExperts.loading,
    // loading, available, selected,
  },
  extraReducers(builder) {
    builder.addCase(getExpertsThunk.pending, (state) => {
      state.loading = true;
      state.error = null;
      // state.workspace = action.meta.arg.located_fgroup_id;
    });
    builder.addCase(getExpertsThunk.rejected, (state, action) => {
      // TODO: global error
      state.loading = false;
      state.error = action.payload?.message ?? null;
    });

    builder.addCase(getExpertsThunk.fulfilled, (state, action) => {
      state.loading = false;
      state.error = null;
      state.experts = action.payload.experts_effective_list;
      // TODO: if store model isn't supported?
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
        state.selectedExpert = action.payload.experts_effective_list[0].fexp_id;
      }
    });

    builder.addCase(getModelsForExpertThunk.pending, (state, action) => {
      // TODO: action.meta.arg.inside_fgroup_id
      if (!(action.meta.arg.fexp_id in state.modelsForExperts.models)) {
        state.modelsForExperts.loading = true;
      }
    });

    // TODO: global error
    builder.addCase(getModelsForExpertThunk.rejected, (state, action) => {
      state.modelsForExperts.loading = false;
      state.error = action.error.message ?? null;
    });

    builder.addCase(getModelsForExpertThunk.fulfilled, (state, action) => {
      state.modelsForExperts.loading = false;
      state.modelsForExperts.models[action.meta.arg.fexp_id] =
        action.payload.expert_choice_consequences;

      const names = action.payload.expert_choice_consequences.map(
        (model) => model.provm_name,
      );
      if (!state.selectedModel && names.length > 0) {
        state.selectedModel = names[0];
      }
      if (state.selectedModel && !names.includes(state.selectedModel)) {
        state.selectedModel = null;
      }
    });

    builder.addCase(setCurrentProjectInfo, () => {
      return initialState;
    });
  },
});

export const {
  selectCurrentExpert,
  selectAvailableExperts,
  selectCurrentModel,
  selectIsExpertsLoading,
  selectModelsForExpert,
  selectModelsForExpertLoading,
} = expertsSlice.selectors;

export const { setExpert, setModel } = expertsSlice.actions;
