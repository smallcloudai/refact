import { createSlice, PayloadAction } from "@reduxjs/toolkit";

export type AgentUsageMeta = {
  agent_usage: null | number; // null if plan is PRO or ROBOT
  agent_max_usage_amount: number; // maximum amount of agent usage allowed per UTC day for users with FREE plan
};

const initialState: AgentUsageMeta = {
  agent_usage: null,
  agent_max_usage_amount: 20,
};

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

  selectors: {
    selectAgentUsage: (state) => state.agent_usage,
    selectMaxAgentUsageAmount: (state) => state.agent_max_usage_amount,
  },
});

export const {
  updateAgentUsage,
  updateMaxAgentUsageAmount,
  setInitialAgentUsage,
} = agentUsageSlice.actions;
export const { selectAgentUsage, selectMaxAgentUsageAmount } =
  agentUsageSlice.selectors;
