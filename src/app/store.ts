import {
  combineSlices,
  configureStore,
  // createSlice,
} from "@reduxjs/toolkit";
import { storage } from "./storage";
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
  pathApi,
  pingApi,
} from "../services/refact";
import { smallCloudApi } from "../services/smallcloud";
import { reducer as fimReducer } from "../features/FIM/reducer";
import { tourReducer } from "../features/Tour";
import { tipOfTheDayReducer } from "../features/TipOfTheDay";
import { reducer as configReducer } from "../features/Config/configSlice";
import { activeFileReducer } from "../features/Chat/activeFile";
import { selectedSnippetReducer } from "../features/Chat/selectedSnippet";
import { chatReducer } from "../features/Chat/Thread/reducer";
import {
  historySlice,
  historyMiddleware,
} from "../features/History/historySlice";
import { errorSlice } from "../features/Errors/errorsSlice";
import { warningSlice } from "../features/Errors/warningSlice";
import { pagesSlice } from "../features/Pages/pagesSlice";
import mergeInitialState from "redux-persist/lib/stateReconciler/autoMergeLevel2";
import { listenerMiddleware } from "./middleware";

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
    [smallCloudApi.reducerPath]: smallCloudApi.reducer,
    [pathApi.reducerPath]: pathApi.reducer,
    [pingApi.reducerPath]: pingApi.reducer,
  },
  historySlice,
  errorSlice,
  warningSlice,
  pagesSlice,
);

const persistConfig = {
  key: "root",
  storage: storage(),
  whitelist: [historySlice.reducerPath, "tour", "tipOfTheDay"],
  stateReconciler: mergeInitialState,
};

const persistedReducer = persistReducer<ReturnType<typeof rootReducer>>(
  persistConfig,
  rootReducer,
);

export type RootState = ReturnType<typeof persistedReducer>;

export function setUpStore(preloadedState?: Partial<RootState>) {
  const initialState = {
    ...preloadedState,
    ...window.__INITIAL_STATE__,
  } as RootState;

  const store = configureStore({
    reducer: persistedReducer,
    preloadedState: initialState,
    devTools: {
      maxAge: 1000,
    },
    middleware: (getDefaultMiddleware) => {
      return (
        getDefaultMiddleware({
          serializableCheck: {
            ignoredActions: [FLUSH, REHYDRATE, PAUSE, PERSIST, PURGE, REGISTER],
          },
        })
          .concat(
            pingApi.middleware,
            statisticsApi.middleware,
            capsApi.middleware,
            promptsApi.middleware,
            toolsApi.middleware,
            commandsApi.middleware,
            diffApi.middleware,
            smallCloudApi.middleware,
            pathApi.middleware,
          )
          .prepend(historyMiddleware.middleware)
          // .prepend(errorMiddleware.middleware)
          .prepend(listenerMiddleware.middleware)
      );
    },
  });

  return store;
}
export const store = setUpStore();
export type Store = typeof store;

export const persistor = persistStore(store);
// TODO: sync storage accross windows (was buggy when deleting).
// window.onstorage = (event) => {
//   if (!event.key || !event.key.endsWith(persistConfig.key)) {
//     return;
//   }

//   if (event.oldValue === event.newValue) {
//     return;
//   }
//   if (event.newValue === null) {
//     return;
//   }

//   const statePartial = parseOrElse<Record<string, string>>(event.newValue, {});

//   const state = Object.keys(statePartial).reduce<Record<string, unknown>>(
//     (acc, reducerKey) => {
//       if (!persistConfig.whitelist.includes(reducerKey)) {
//         return acc;
//       }

//       if (!(reducerKey in statePartial)) {
//         return acc;
//       }

//       const itemAsString = statePartial[reducerKey];
//       acc[reducerKey] = JSON.parse(itemAsString);

//       return acc;
//     },
//     {},
//   );

//   store.dispatch({
//     type: REHYDRATE,
//     key: persistConfig.key,
//     payload: state,
//   });
// };

// Infer the `RootState` and `AppDispatch` types from the store itself
// export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;

// Infer the type of `store`
export type AppStore = typeof store;

declare global {
  interface Window {
    __INITIAL_STATE__?: RootState;
  }
}
