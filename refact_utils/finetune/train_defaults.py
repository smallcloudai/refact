finetune_train_defaults = {
    "autoselect_test_files_num": 3,
    "model_ctx_size": 1024,
    "filter_loss_threshold": 3.0,
    "trainable_embeddings": False,
    "low_gpu_mem_mode": True,
    "lr": 30e-5,
    "batch_size": 128,
    "warmup_num_steps": 20,
    "weight_decay": 0.1,
    "lora_r": 16,
    "lora_alpha": 32,
    "lora_dropout": 0.01,
    # if train_steps==0 then set_train_steps() and  set_lr_decay_steps() is automatic
    "train_steps": 0,
    "lr_decay_steps": 0,
}
