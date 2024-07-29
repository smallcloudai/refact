import {
  combineSlices,
  configureStore,
  // createSlice,
  // combineSlices,
} from "@reduxjs/toolkit";
import { statisticsApi } from "../services/refact/statistics";
import { capsApi } from "../services/refact/caps";
import { reducer as fimReducer } from "../features/FIM/reducer";
// import { fimSlice } from "../features/FIM/fimSlice";

// https://redux-toolkit.js.org/api/combineSlices
// `combineSlices` automatically combines the reducers using
// their `reducerPath`s, therefore we no longer need to call `combineReducers`.
const rootReducer = combineSlices({
  fim: fimReducer,
  [statisticsApi.reducerPath]: statisticsApi.reducer,
  [capsApi.reducerPath]: capsApi.reducer,
});

// Infer the `RootState` type from the root reducer

export const store = configureStore({
  reducer: rootReducer,
  middleware: (getDefaultMiddleware) => {
    return getDefaultMiddleware().concat(
      statisticsApi.middleware,
      capsApi.middleware,
    );
  },
});

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;

// Infer the type of `store`
export type AppStore = typeof store;
