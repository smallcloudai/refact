import { createSlice, type PayloadAction } from "@reduxjs/toolkit";
import {
  IntegrationPrimitive,
  Integration,
} from "../../services/refact/integrations";

type FormKeyValueMap = Integration["integr_values"];

export type IntegrationCachedFormData = Record<string, FormKeyValueMap>;

const initialState: { cachedForms: IntegrationCachedFormData } = {
  cachedForms: {},
};

export const integrationsSlice = createSlice({
  name: "integrations",
  initialState,
  reducers: {
    addToCacheOnMiss: (state, action: PayloadAction<Integration>) => {
      const key = action.payload.integr_config_path;
      if (key in state.cachedForms) return state;

      state.cachedForms[key] = action.payload.integr_values;
    },
    //TODO: could just be the path
    removeFromCache: (state, action: PayloadAction<string>) => {
      if (!(action.payload in state.cachedForms)) return state;

      const nextCache = Object.entries(
        state.cachedForms,
      ).reduce<IntegrationCachedFormData>((acc, [curKey, curValues]) => {
        if (curKey === action.payload) return acc;
        return { ...acc, [curKey]: curValues };
      }, {});

      state.cachedForms = nextCache;
    },

    clearCache: (state) => {
      state.cachedForms = {};
    },
  },
  selectors: {
    maybeSelectIntegrationFromCache: (state, integration: Integration) => {
      if (!(integration.integr_config_path in state.cachedForms)) return null;
      return state.cachedForms[integration.integr_config_path];
    },

    checkValuesForChanges:
      (_state, _integration: Integration) =>
      (_accessors: string | string[], _value: IntegrationPrimitive) => {
        // TODO: maybe add this ??
        return false;
      },
  },
});

export const { addToCacheOnMiss, removeFromCache } = integrationsSlice.actions;
export const { maybeSelectIntegrationFromCache, checkValuesForChanges } =
  integrationsSlice.selectors;
