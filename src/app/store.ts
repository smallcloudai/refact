import {
  combineSlices,
  configureStore,
  // createSlice,
} from "@reduxjs/toolkit";
import storage from "redux-persist/lib/storage";
import {
  FLUSH,
  PAUSE,
  PERSIST,
  PURGE,
  REGISTER,
  REHYDRATE,
  persistReducer,
  persistStore,
} from "redux-persist";
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
import { reducer as configReducer } from "../features/Config/reducer";
import { activeFileReducer } from "../features/Chat/activeFile";
import { selectedSnippetReducer } from "../features/Chat/selectedSnippet";
import { chatReducer } from "../features/Chat/chatThread";
import {
  historySlice,
  historyMiddleware,
} from "../features/History/historySlice";

// https://redux-toolkit.js.org/api/combineSlices
// `combineSlices` automatically combines the reducers using
// their `reducerPath`s, therefore we no longer need to call `combineReducers`.
const rootReducer = combineSlices(
  {
    fim: fimReducer,
    tour: tourReducer,
    tipOfTheDay: tipOfTheDayReducer,
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
    return getDefaultMiddleware({
      serializableCheck: {
        ignoredActions: [FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER],
      },
    })
      .concat(
        statisticsApi.middleware,
        capsApi.middleware,
        promptsApi.middleware,
        toolsApi.middleware,
        commandsApi.middleware,
        diffApi.middleware,
      )
      .prepend(historyMiddleware.middleware);
  },
});

store.subscribe(() => {
  saveTourToLocalStorage(store.getState());
  saveTipOfTheDayToLocalStorage(store.getState());
});

export const persistor = persistStore(store);

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;

// Infer the type of `store`
export type AppStore = typeof store;
