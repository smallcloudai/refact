import {
  configureStore,
  // combineSlices,
} from "@reduxjs/toolkit";
import { statisticsApi } from "../events";

// `combineSlices` automatically combines the reducers using
// their `reducerPath`s, therefore we no longer need to call `combineReducers`.
// const rootReducer = combineSlices(statisticsSlice);
// Infer the `RootState` type from the root reducer
// export type RootState = ReturnType<typeof rootReducer>;

export const store = configureStore({
  reducer: {
    [statisticsApi.reducerPath]: statisticsApi.reducer,
  },
  middleware: (getDefaultMiddleware) => {
    return getDefaultMiddleware().concat(statisticsApi.middleware);
  },
});

// Infer the `RootState` and `AppDispatch` types from the store itself
export type RootState = ReturnType<typeof store.getState>;
// Inferred type: {posts: PostsState, comments: CommentsState, users: UsersState}
export type AppDispatch = typeof store.dispatch;

// Infer the type of `store`
export type AppStore = typeof store;
