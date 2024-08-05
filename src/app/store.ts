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
import { reducer as configReducer } from "../features/Config/reducer";
import { activeFileReducer } from "../features/Chat2/activeFile";
import { selectedSnippetReducer } from "../features/Chat2/selectedSnippet";

// import { fimSlice } from "../features/FIM/fimSlice";

// https://redux-toolkit.js.org/api/combineSlices
// `combineSlices` automatically combines the reducers using
// their `reducerPath`s, therefore we no longer need to call `combineReducers`.
const rootReducer = combineSlices({
  fim: fimReducer,
  config: configReducer,
  active_file: activeFileReducer,
  selected_snippet: selectedSnippetReducer,
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

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;

// Infer the type of `store`
export type AppStore = typeof store;
