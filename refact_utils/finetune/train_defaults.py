finetune_train_defaults = {
    "limit_time_seconds": 48 * 60 * 60,
    "lr": 30e-5,
    "batch_size": 128,
    "warmup_num_steps": 20,
    "weight_decay": 0.1,
    "use_heuristics": True,
    # These settings will be used if it's use_heuristics=False
    "train_steps": 250,
    "lr_decay_steps": 250,
    "lora_r": 16,
    "lora_alpha": 32,
    "lora_dropout": 0.01,
    "trainable_embeddings": False,
    "low_gpu_mem_mode": True
}
