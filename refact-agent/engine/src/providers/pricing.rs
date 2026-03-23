use crate::call_validation::{ChatUsage, MeteringUsd};
use crate::providers::traits::ModelPricing;

pub fn compute_cost(usage: &ChatUsage, pricing: &ModelPricing) -> Option<MeteringUsd> {
    if !pricing.is_valid() {
        return None;
    }

    let prompt_usd = (usage.prompt_tokens as f64) * pricing.prompt / 1_000_000.0;
    let generated_usd = (usage.completion_tokens as f64) * pricing.generated / 1_000_000.0;

    let cache_read_usd = match (usage.cache_read_tokens, pricing.cache_read) {
        (Some(tokens), Some(rate)) => Some((tokens as f64) * rate / 1_000_000.0),
        _ => None,
    };

    let cache_creation_usd = match (usage.cache_creation_tokens, pricing.cache_creation) {
        (Some(tokens), Some(rate)) => Some((tokens as f64) * rate / 1_000_000.0),
        _ => None,
    };

    let total_usd = prompt_usd + generated_usd
        + cache_read_usd.unwrap_or(0.0)
        + cache_creation_usd.unwrap_or(0.0);

    Some(MeteringUsd {
        prompt_usd,
        generated_usd,
        cache_read_usd,
        cache_creation_usd,
        total_usd,
    })
}


pub fn openai_pricing(model_id: &str) -> Option<ModelPricing> {
    let id = model_id.to_lowercase();
    match id.as_str() {
        // Codex-mini (standalone model, not gpt-5 family)
        s if s.contains("codex-mini") => Some(ModelPricing {
            prompt: 1.50,
            generated: 6.00,
            cache_read: Some(0.375),
            cache_creation: None,
        }),
        // GPT-5 family (also covers gpt-5-codex, gpt-5.1-codex, gpt-5.2-codex)
        s if s.contains("gpt-5") && s.contains("nano") => Some(ModelPricing {
            prompt: 0.05,
            generated: 0.40,
            cache_read: Some(0.005),
            cache_creation: None,
        }),
        s if s.contains("gpt-5") && s.contains("mini") => Some(ModelPricing {
            prompt: 0.25,
            generated: 2.00,
            cache_read: Some(0.025),
            cache_creation: None,
        }),
        s if s.contains("gpt-5") && s.contains("pro") => Some(ModelPricing {
            prompt: 15.00,
            generated: 120.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("gpt-5") => Some(ModelPricing {
            prompt: 1.25,
            generated: 10.00,
            cache_read: Some(0.125),
            cache_creation: None,
        }),
        // GPT-4.1 family
        s if s.contains("gpt-4.1-nano") || s.contains("gpt-4.1_nano") => Some(ModelPricing {
            prompt: 0.10,
            generated: 0.40,
            cache_read: Some(0.025),
            cache_creation: None,
        }),
        s if s.contains("gpt-4.1-mini") || s.contains("gpt-4.1_mini") => Some(ModelPricing {
            prompt: 0.40,
            generated: 1.60,
            cache_read: Some(0.10),
            cache_creation: None,
        }),
        s if s.contains("gpt-4.1") => Some(ModelPricing {
            prompt: 2.00,
            generated: 8.00,
            cache_read: Some(0.50),
            cache_creation: None,
        }),
        // GPT-4o family
        s if s.contains("gpt-4o-mini") => Some(ModelPricing {
            prompt: 0.15,
            generated: 0.60,
            cache_read: Some(0.075),
            cache_creation: None,
        }),
        s if s.contains("gpt-4o") => Some(ModelPricing {
            prompt: 2.50,
            generated: 10.00,
            cache_read: Some(1.25),
            cache_creation: None,
        }),
        // Legacy GPT-4
        s if s.contains("gpt-4-turbo") => Some(ModelPricing {
            prompt: 10.00,
            generated: 30.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("gpt-4") && !s.contains("turbo") && !s.contains("4o") && !s.contains("4.1") => Some(ModelPricing {
            prompt: 30.00,
            generated: 60.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("gpt-3.5-turbo") => Some(ModelPricing {
            prompt: 0.50,
            generated: 1.50,
            cache_read: None,
            cache_creation: None,
        }),
        // o4-mini
        s if s.contains("o4-mini") => Some(ModelPricing {
            prompt: 1.10,
            generated: 4.40,
            cache_read: Some(0.275),
            cache_creation: None,
        }),
        // o3 family
        s if s.contains("o3-pro") => Some(ModelPricing {
            prompt: 20.00,
            generated: 80.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("o3-mini") => Some(ModelPricing {
            prompt: 1.10,
            generated: 4.40,
            cache_read: Some(0.55),
            cache_creation: None,
        }),
        s if s.contains("o3") => Some(ModelPricing {
            prompt: 2.00,
            generated: 8.00,
            cache_read: Some(0.50),
            cache_creation: None,
        }),
        // o1 family
        s if s.contains("o1-pro") => Some(ModelPricing {
            prompt: 150.00,
            generated: 600.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("o1-mini") => Some(ModelPricing {
            prompt: 1.10,
            generated: 4.40,
            cache_read: Some(0.55),
            cache_creation: None,
        }),
        s if s.contains("o1") => Some(ModelPricing {
            prompt: 15.00,
            generated: 60.00,
            cache_read: Some(7.50),
            cache_creation: None,
        }),
        _ => None,
    }
}

pub fn anthropic_pricing(model_id: &str) -> Option<ModelPricing> {
    let id = model_id.to_lowercase();
    match id.as_str() {
        // Claude Opus 4.5/4.6
        s if s.contains("claude-opus-4") && (s.contains("4.5") || s.contains("4.6") || s.contains("4-5") || s.contains("4-6")) => Some(ModelPricing {
            prompt: 5.00,
            generated: 25.00,
            cache_read: Some(0.50),
            cache_creation: Some(6.25),
        }),
        // Claude Opus 4/4.1
        s if s.contains("claude-opus-4") => Some(ModelPricing {
            prompt: 15.00,
            generated: 75.00,
            cache_read: Some(1.50),
            cache_creation: Some(18.75),
        }),
        // Claude 3 Opus (legacy)
        s if s.contains("claude-3-opus") => Some(ModelPricing {
            prompt: 15.00,
            generated: 75.00,
            cache_read: Some(1.50),
            cache_creation: Some(18.75),
        }),
        // Claude Sonnet 4/4.5
        s if s.contains("claude-sonnet-4") => Some(ModelPricing {
            prompt: 3.00,
            generated: 15.00,
            cache_read: Some(0.30),
            cache_creation: Some(3.75),
        }),
        // Claude 3.7 Sonnet (deprecated but still used)
        s if s.contains("claude-3-7-sonnet") || s.contains("claude-3.7-sonnet") => Some(ModelPricing {
            prompt: 3.00,
            generated: 15.00,
            cache_read: Some(0.30),
            cache_creation: Some(3.75),
        }),
        // Claude 3.5 Sonnet
        s if s.contains("claude-3-5-sonnet") || s.contains("claude-3.5-sonnet") => Some(ModelPricing {
            prompt: 3.00,
            generated: 15.00,
            cache_read: Some(0.30),
            cache_creation: Some(3.75),
        }),
        // Claude Haiku 4.5
        s if s.contains("claude-haiku-4") && s.contains("4.5") => Some(ModelPricing {
            prompt: 1.00,
            generated: 5.00,
            cache_read: Some(0.10),
            cache_creation: Some(1.25),
        }),
        // Claude 3.5 Haiku
        s if s.contains("claude-3-5-haiku") || s.contains("claude-3.5-haiku") => Some(ModelPricing {
            prompt: 0.80,
            generated: 4.00,
            cache_read: Some(0.08),
            cache_creation: Some(1.00),
        }),
        // Claude 3 Haiku
        s if s.contains("claude-3-haiku") => Some(ModelPricing {
            prompt: 0.25,
            generated: 1.25,
            cache_read: Some(0.03),
            cache_creation: Some(0.30),
        }),
        _ => None,
    }
}

pub fn google_gemini_pricing(model_id: &str) -> Option<ModelPricing> {
    let id = model_id.to_lowercase();
    match id.as_str() {
        // Gemini 3 family (preview)
        s if s.contains("gemini-3") && s.contains("pro") => Some(ModelPricing {
            prompt: 2.00,
            generated: 12.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("gemini-3") && s.contains("flash") => Some(ModelPricing {
            prompt: 0.50,
            generated: 3.00,
            cache_read: None,
            cache_creation: None,
        }),
        // Gemini 2.5 family
        s if s.contains("gemini-2.5-pro") || s.contains("gemini-2-5-pro") => Some(ModelPricing {
            prompt: 1.25,
            generated: 10.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("gemini-2.5-flash-lite") || s.contains("gemini-2-5-flash-lite") => Some(ModelPricing {
            prompt: 0.10,
            generated: 0.40,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("gemini-2.5-flash") || s.contains("gemini-2-5-flash") => Some(ModelPricing {
            prompt: 0.30,
            generated: 2.50,
            cache_read: None,
            cache_creation: None,
        }),
        // Gemini 2.0 family
        s if s.contains("gemini-2.0-flash-lite") || s.contains("gemini-2-0-flash-lite") => Some(ModelPricing {
            prompt: 0.10,
            generated: 0.40,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("gemini-2.0-flash") || s.contains("gemini-2-flash") => Some(ModelPricing {
            prompt: 0.10,
            generated: 0.40,
            cache_read: None,
            cache_creation: None,
        }),
        // Gemini 1.5 family
        s if s.contains("gemini-1.5-flash") || s.contains("gemini-flash") => Some(ModelPricing {
            prompt: 0.075,
            generated: 0.30,
            cache_read: Some(0.01875),
            cache_creation: Some(0.01875),
        }),
        s if s.contains("gemini-1.5-pro") || s.contains("gemini-pro") => Some(ModelPricing {
            prompt: 1.25,
            generated: 5.00,
            cache_read: Some(0.3125),
            cache_creation: Some(0.3125),
        }),
        _ => None,
    }
}

pub fn deepseek_pricing(model_id: &str) -> Option<ModelPricing> {
    let id = model_id.to_lowercase();
    match id.as_str() {
        s if s.contains("deepseek-chat") || s.contains("deepseek-v3") => Some(ModelPricing {
            prompt: 0.27,
            generated: 1.10,
            cache_read: Some(0.07),
            cache_creation: None,
        }),
        s if s.contains("deepseek-reasoner") || s.contains("deepseek-r1") => Some(ModelPricing {
            prompt: 0.55,
            generated: 2.19,
            cache_read: Some(0.14),
            cache_creation: None,
        }),
        s if s.contains("deepseek-coder") => Some(ModelPricing {
            prompt: 0.14,
            generated: 0.28,
            cache_read: None,
            cache_creation: None,
        }),
        _ => None,
    }
}

pub fn xai_pricing(model_id: &str) -> Option<ModelPricing> {
    let id = model_id.to_lowercase();
    match id.as_str() {
        s if s.contains("grok-3") => Some(ModelPricing {
            prompt: 3.00,
            generated: 15.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("grok-2") => Some(ModelPricing {
            prompt: 2.00,
            generated: 10.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("grok-beta") || s.contains("grok") => Some(ModelPricing {
            prompt: 5.00,
            generated: 15.00,
            cache_read: None,
            cache_creation: None,
        }),
        _ => None,
    }
}

pub fn groq_pricing(model_id: &str) -> Option<ModelPricing> {
    let id = model_id.to_lowercase();
    match id.as_str() {
        s if s.contains("llama-3.3-70b") => Some(ModelPricing {
            prompt: 0.59,
            generated: 0.79,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("llama-3.1-70b") => Some(ModelPricing {
            prompt: 0.59,
            generated: 0.79,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("llama-3.1-8b") || s.contains("llama-3-8b") => Some(ModelPricing {
            prompt: 0.05,
            generated: 0.08,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("mixtral-8x7b") => Some(ModelPricing {
            prompt: 0.24,
            generated: 0.24,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("gemma2-9b") || s.contains("gemma-9b") => Some(ModelPricing {
            prompt: 0.20,
            generated: 0.20,
            cache_read: None,
            cache_creation: None,
        }),
        _ => None,
    }
}

pub fn minimax_pricing(model_id: &str) -> Option<ModelPricing> {
    let id = model_id.to_lowercase();
    match id.as_str() {
        s if s.contains("m2.7") || s.contains("m2-7") => Some(ModelPricing {
            prompt: 1.00,
            generated: 4.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("m2.5") && s.contains("highspeed") || s.contains("m2-5") && s.contains("highspeed") => Some(ModelPricing {
            prompt: 0.80,
            generated: 3.00,
            cache_read: None,
            cache_creation: None,
        }),
        s if s.contains("m2.5") || s.contains("m2-5") => Some(ModelPricing {
            prompt: 0.80,
            generated: 3.00,
            cache_read: None,
            cache_creation: None,
        }),
        _ => None,
    }
}
