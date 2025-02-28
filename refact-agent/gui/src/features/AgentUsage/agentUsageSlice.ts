import { createSlice, PayloadAction } from "@reduxjs/toolkit";
import { smallCloudApi } from "../../services/smallcloud";
import { chatResponse } from "../Chat";
import { isChatResponseChoice } from "../../services/refact/types";

export type AgentUsageMeta = {
  agent_usage: null | number; // null if plan is PRO or ROBOT
  agent_max_usage_amount: number; // maximum amount of agent usage allowed per UTC day for users with FREE plan
};

const initialState: AgentUsageMeta = {
  agent_usage: 0,
  agent_max_usage_amount: 8,
};

// TODO: is this needed since agent usage now comes from the getUser request and chat response :/ ?
export const agentUsageSlice = createSlice({
  name: "agentUsage",
  initialState,
  reducers: {
    updateAgentUsage: (
      state,
      action: PayloadAction<AgentUsageMeta["agent_usage"]>,
    ) => {
      state.agent_usage = action.payload;
    },
    updateMaxAgentUsageAmount: (state, action: PayloadAction<number>) => {
      state.agent_max_usage_amount = action.payload;
    },
    setInitialAgentUsage: (state, action: PayloadAction<AgentUsageMeta>) => {
      const { agent_max_usage_amount, agent_usage } = action.payload;
      state.agent_usage = agent_usage;
      state.agent_max_usage_amount = agent_max_usage_amount;
    },
  },

  extraReducers: (builder) => {
    builder.addMatcher(
      smallCloudApi.endpoints.getUser.matchFulfilled,
      (state, action) => {
        const { refact_agent_max_request_num, refact_agent_request_available } =
          action.payload;
        state.agent_max_usage_amount = refact_agent_max_request_num;
        state.agent_usage = refact_agent_request_available;
      },
    );
  },

  selectors: {
    selectAgentUsage: (state) => state.agent_usage,
    selectMaxAgentUsageAmount: (state) => state.agent_max_usage_amount,
  },

  extraReducers: (builder) => {
    builder.addMatcher(
      smallCloudApi.endpoints.getUser.matchFulfilled,
      (state, action) => {
        // update logic here
        state.agent_max_usage_amount =
          action.payload.refact_agent_max_request_num;
        state.agent_usage = action.payload.refact_agent_request_available;
      },
    );

    builder.addMatcher(chatResponse.match, (state, action) => {
      if (!isChatResponseChoice(action.payload)) return state;
      const { refact_agent_max_request_num, refact_agent_request_available } =
        action.payload;

      state.agent_max_usage_amount =
        refact_agent_max_request_num ?? state.agent_max_usage_amount;
      state.agent_usage = refact_agent_request_available ?? state.agent_usage;
    });
  },
});

export const {
  setInitialAgentUsage,
  updateAgentUsage,
  updateMaxAgentUsageAmount,
} = agentUsageSlice.actions;
export const { selectAgentUsage, selectMaxAgentUsageAmount } =
  agentUsageSlice.selectors;
