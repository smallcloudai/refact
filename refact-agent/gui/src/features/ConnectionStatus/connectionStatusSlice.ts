import { createSlice, PayloadAction } from "@reduxjs/toolkit/react";

type InitialState = {
  connections: Record<
    string,
    { id: string; name: string; status: "connecting" | "connected" }
  >;
};

const initialState: InitialState = {
  connections: {},
};

export const connectionStatusSlice = createSlice({
  name: "connection_status",
  initialState,
  reducers: {
    connecting: (
      state,
      action: PayloadAction<{ id: string; name: string }>,
    ) => {
      state.connections[action.payload.id] = {
        ...action.payload,
        status: "connecting",
      };
    },
    connected: (state, action: PayloadAction<{ id: string; name: string }>) => {
      state.connections[action.payload.id] = {
        ...action.payload,
        status: "connecting",
      };
    },
    closed: (state, action: PayloadAction<{ id: string }>) => {
      // state.connections[action.payload.id] = "closed";
      const others = Object.entries(state.connections).reduce<
        InitialState["connections"]
      >((acc, [key, value]) => {
        if (key === action.payload.id) return acc;
        return { ...acc, [key]: value };
      }, {});

      state.connections = others;
    },
  },

  selectors: {
    selectConnections: (state) => Object.values(state.connections),
  },
});

export const { connected, connecting, closed } = connectionStatusSlice.actions;
export const { selectConnections } = connectionStatusSlice.selectors;
