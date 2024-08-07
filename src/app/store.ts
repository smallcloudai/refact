import {
  combineSlices,
  configureStore,
  // createSlice,
} from "@reduxjs/toolkit";
import storage from "redux-persist/lib/storage";
import { persistReducer, persistStore } from "redux-persist";
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
import { chatReducer } from "../features/Chat2/chatThread";
import { historySlice } from "../features/History/historySlice";

// import { fimSlice } from "../features/FIM/fimSlice";

// https://redux-toolkit.js.org/api/combineSlices
// `combineSlices` automatically combines the reducers using
// their `reducerPath`s, therefore we no longer need to call `combineReducers`.
const rootReducer = combineSlices(
  {
    fim: fimReducer,
    config: configReducer,
    active_file: activeFileReducer,
    selected_snippet: selectedSnippetReducer,
    chat: chatReducer,
    [statisticsApi.reducerPath]: statisticsApi.reducer,
    [capsApi.reducerPath]: capsApi.reducer,
    [promptsApi.reducerPath]: promptsApi.reducer,
    [toolsApi.reducerPath]: toolsApi.reducer,
    [commandsApi.reducerPath]: commandsApi.reducer,
    [diffApi.reducerPath]: diffApi.reducer,
  },
  historySlice,
);

const persistConfig = {
  key: "root",
  storage,
  whitelist: [historySlice.reducerPath],
};

const persistedReducer = persistReducer(persistConfig, rootReducer);

export const store = configureStore({
  reducer: persistedReducer,
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

export const persistor = persistStore(store);

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;

// Infer the type of `store`
export type AppStore = typeof store;
