import {
  buildCreateSlice,
  asyncThunkCreator,
  SerializedError,
} from "@reduxjs/toolkit";
import {
  BasicStuffDocument,
  BasicStuffQuery,
  BasicStuffQueryVariables,
} from "../../../generated/documents";

import { createGraphqlClient } from "../../services/graphql/createClient";

const createAppSlice = buildCreateSlice({
  creators: { asyncThunk: asyncThunkCreator },
});

type InitialState = {
  loading: boolean;
  error: null | SerializedError;
  data: null | BasicStuffQuery["query_basic_stuff"];
};

const initialState: InitialState = {
  loading: false,
  error: null,
  data: null,
};

export const basicStuffSlice = createAppSlice({
  name: "basic_stuff",
  initialState: initialState,
  reducers: (creators) => ({
    resetBasicStuff: creators.reducer((state) => {
      state = initialState;
      return state;
    }),
    getBasicStuff: creators.asyncThunk(
      async (args: { apiKey: string; addressUrl: string }, thunkAPI) => {
        const client = createGraphqlClient(
          args.addressUrl,
          args.apiKey,
          thunkAPI.signal,
        );

        const result = await client.query<
          BasicStuffQuery,
          BasicStuffQueryVariables
        >(BasicStuffDocument, {});
        return result;
      },
      {
        pending: (state) => {
          state.error = null;
          state.loading = true;
        },
        rejected: (state, action) => {
          state.error = action.payload ?? action.error;
        },
        fulfilled: (state, action) => {
          state.data = action.payload.data?.query_basic_stuff ?? null;
        },
        settled: (state, _action) => {
          state.loading = false;
        },
      },
    ),
  }),

  selectors: {
    selectBasicStuffSlice: (state) => state,
  },
  // TODO: errors
});

export const { getBasicStuff, resetBasicStuff } = basicStuffSlice.actions;
export const { selectBasicStuffSlice } = basicStuffSlice.selectors;
