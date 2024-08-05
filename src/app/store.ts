import {
  combineSlices,
  configureStore,
  // createSlice,
} from "@reduxjs/toolkit";
import { statisticsApi } from "../services/refact/statistics";
import {
  capsApi,
  promptsApi,
  toolsApi,
  commandsApi,
  diffApi,
} from "../services/refact";
import { reducer as fimReducer } from "../features/FIM/reducer";
import { saveTourToLocalStorage, tourReducer } from "../features/Tour";
import {
  saveTipOfTheDayToLocalStorage,
  tipOfTheDayReducer,
} from "../features/TipOfTheDay";
// import { fimSlice } from "../features/FIM/fimSlice";

// https://redux-toolkit.js.org/api/combineSlices
// `combineSlices` automatically combines the reducers using
// their `reducerPath`s, therefore we no longer need to call `combineReducers`.
const rootReducer = combineSlices({
  fim: fimReducer,
  tour: tourReducer,
  tipOfTheDay: tipOfTheDayReducer,
  [statisticsApi.reducerPath]: statisticsApi.reducer,
  [capsApi.reducerPath]: capsApi.reducer,
  [promptsApi.reducerPath]: promptsApi.reducer,
  [toolsApi.reducerPath]: toolsApi.reducer,
  [commandsApi.reducerPath]: commandsApi.reducer,
  [diffApi.reducerPath]: diffApi.reducer,
});

// Infer the `RootState` type from the root reducer

export const store = configureStore({
  reducer: rootReducer,
  middleware: (getDefaultMiddleware) => {
    return getDefaultMiddleware().concat(
      statisticsApi.middleware,
      capsApi.middleware,
      promptsApi.middleware,
      toolsApi.middleware,
      commandsApi.middleware,
      diffApi.middleware,
    );
  },
});

store.subscribe(() => {
  saveTourToLocalStorage(store.getState());
  saveTipOfTheDayToLocalStorage(store.getState());
});

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;

// Infer the type of `store`
export type AppStore = typeof store;
