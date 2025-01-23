import { combineSlices, configureStore } from "@reduxjs/toolkit";
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
  integrationsApi,
  dockerApi,
  telemetryApi,
} from "../services/refact";
import { smallCloudApi } from "../services/smallcloud";
import { reducer as fimReducer } from "../features/FIM/reducer";
import { tourReducer } from "../features/Tour";
import { tipOfTheDaySlice } from "../features/TipOfTheDay";
import { reducer as configReducer } from "../features/Config/configSlice";
import { activeFileReducer } from "../features/Chat/activeFile";
import { selectedSnippetReducer } from "../features/Chat/selectedSnippet";
import { chatReducer } from "../features/Chat/Thread/reducer";
import {
  historySlice,
  historyMiddleware,
} from "../features/History/historySlice";
import { errorSlice } from "../features/Errors/errorsSlice";

import { pagesSlice } from "../features/Pages/pagesSlice";
import mergeInitialState from "redux-persist/lib/stateReconciler/autoMergeLevel2";
import { listenerMiddleware } from "./middleware";
import { informationSlice } from "../features/Errors/informationSlice";
import { confirmationSlice } from "../features/ToolConfirmation/confirmationSlice";
import { attachedImagesSlice } from "../features/AttachedImages";
import { userSurveySlice } from "../features/UserSurvey/userSurveySlice";
import { linksApi } from "../services/refact/links";
import { integrationsSlice } from "../features/Integrations";
import { agentUsageSlice } from "../features/AgentUsage/agentUsageSlice";

const tipOfTheDayPersistConfig = {
  key: "totd",
  storage: storage(),
  stateReconciler: mergeInitialState,
};

const agentUsagePersistConfig = {
  key: "agentUsage",
  storage: storage(),
  stateReconciler: mergeInitialState,
};

const persistedTipOfTheDayReducer = persistReducer<
  ReturnType<typeof tipOfTheDaySlice.reducer>
>(tipOfTheDayPersistConfig, tipOfTheDaySlice.reducer);

const persistedAgentUsageReducer = persistReducer<
  ReturnType<typeof agentUsageSlice.reducer>
>(agentUsagePersistConfig, agentUsageSlice.reducer);

// https://redux-toolkit.js.org/api/combineSlices
// `combineSlices` automatically combines the reducers using
// their `reducerPath`s, therefore we no longer need to call `combineReducers`.
const rootReducer = combineSlices(
  {
    fim: fimReducer,
    tour: tourReducer,
    // tipOfTheDay: persistedTipOfTheDayReducer,
    [tipOfTheDaySlice.reducerPath]: persistedTipOfTheDayReducer,
    [agentUsageSlice.reducerPath]: persistedAgentUsageReducer,
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
    [linksApi.reducerPath]: linksApi.reducer,
    [telemetryApi.reducerPath]: telemetryApi.reducer,
  },
  historySlice,
  errorSlice,
  informationSlice,
  pagesSlice,
  integrationsApi,
  dockerApi,
  confirmationSlice,
  attachedImagesSlice,
  userSurveySlice,
  integrationsSlice,
);

const rootPersistConfig = {
  key: "root",
  storage: storage(),
  whitelist: [
    historySlice.reducerPath,
    "tour",
    userSurveySlice.reducerPath,
    agentUsageSlice.reducerPath,
  ],
  stateReconciler: mergeInitialState,
};

const persistedReducer = persistReducer<ReturnType<typeof rootReducer>>(
  rootPersistConfig,
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
      maxAge: 50,
    },
    middleware: (getDefaultMiddleware) => {
      const production = import.meta.env.MODE === "production";
      const middleware = production
        ? getDefaultMiddleware({
            thunk: true,
            serializableCheck: false,
            immutableCheck: false,
          })
        : getDefaultMiddleware({
            serializableCheck: {
              ignoredActions: [
                FLUSH,
                REHYDRATE,
                PAUSE,
                PERSIST,
                PURGE,
                REGISTER,
              ],
            },
          });

      return (
        middleware
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
            linksApi.middleware,
            integrationsApi.middleware,
            dockerApi.middleware,
            telemetryApi.middleware,
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
// TODO: sync storage across windows (was buggy when deleting).
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
