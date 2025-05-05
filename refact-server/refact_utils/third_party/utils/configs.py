import litellm

from pydantic import BaseModel, Field
from typing import Dict, Any, Optional

__all__ = [
    "ThirdPartyApiConfig",
    "ProviderConfig",
    "ModelConfig",
    "ModelCapabilities",
]


class ModelCapabilities(BaseModel):
    tools: bool
    multimodal: bool
    agent: bool
    clicks: bool
    completion: bool
    reasoning: Optional[str] = False
    boost_reasoning: bool = False


class ModelConfig(BaseModel):
    model_id: str
    provider_id: str
    api_base: Optional[str]
    api_key: Optional[str]
    n_ctx: int
    max_tokens: int
    capabilities: ModelCapabilities
    tokenizer_id: Optional[str] = None
    extra_headers: Dict[str, str] = Field(default_factory=dict)

    # NOTE: weird function for backward compatibility
    def compose_usage_dict(self, prompt_tokens_n: int, generated_tokens_n: int) -> Dict[str, int]:
        def _pp1000t(cost_entry_name: str) -> int:
            cost = litellm.model_cost.get(self.model_id, {}).get(cost_entry_name, 0)
            return int(cost * 1_000_000 * 1_000)
        return {
            "pp1000t_prompt": _pp1000t("input_cost_per_token"),
            "pp1000t_generated": _pp1000t("output_cost_per_token"),
            "metering_prompt_tokens_n": prompt_tokens_n,
            "metering_generated_tokens_n": generated_tokens_n,
        }

    def to_completion_model_record(self) -> Dict[str, Any]:
        assert self.capabilities.completion
        return {
            "n_ctx": self.n_ctx,
            "supports_scratchpads": {
                "REPLACE_PASSTHROUGH": {
                    "context_format": "chat",
                    "rag_ratio": 0.5,
                }
            },
        }

    def to_chat_model_record(self) -> Dict[str, Any]:
        return {
            "n_ctx": self.n_ctx,
            "supports_scratchpads": {
                "PASSTHROUGH": {},
            },
            "supports_tools": self.capabilities.tools,
            "supports_multimodality": self.capabilities.multimodal,
            "supports_clicks": self.capabilities.clicks,
            "supports_agent": self.capabilities.agent,
            "supports_reasoning": self.capabilities.reasoning,
            "supports_boost_reasoning": self.capabilities.boost_reasoning,
            "default_temperature": 0.6 if self.capabilities.reasoning == "deepseek" else None,
        }


class ProviderConfig(BaseModel):
    enabled: bool = True


class ThirdPartyApiConfig(BaseModel):
    providers: Dict[str, ProviderConfig] = Field(default_factory=dict)
    models: Dict[str, ModelConfig] = Field(default_factory=dict)
