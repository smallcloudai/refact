self.dataopts = DatasetOpts(
    f"n_ctx={cfg['model_info']['ctx_size'] + 1},"
    f"pack_at_most=1,"
    f"quit_on_epoch=1,"
    f"seed=42"
)
self.dataopts.set_encoding(self.model.encoding)



filtered_train = "train_set_filtered.jsonl"
filtered_test = "test_set_filtered.jsonl"




def create_data(model_name, cfg, enc) -> Tuple[Any, Optional[Any]]:
    model_config = supported_models.config[model_name]
    train_dataopts = DatasetOpts(model_config["train_ds_pipeline"]["ds_opts"].format(
        n_ctx=cfg['model_info']['ctx_size'] + 1
    ))
    train_dataopts.set_encoding(enc)
    test_dataopts = DatasetOpts(model_config["test_ds_pipeline"]["ds_opts"].format(
        n_ctx=cfg['model_info']['ctx_size'] + 1
    ))
    test_dataopts.set_encoding(enc)

    train_pipe = getattr(finetune_datasource, model_config["train_ds_pipeline"]["pipeline_name"])
    test_pipe = getattr(finetune_datasource, model_config["test_ds_pipeline"]["pipeline_name"])

    train_ds = train_pipe(filtered_train, train_dataopts)
    train_ds = BatchIterator(train_ds, dataopts=dict(
        batch_size=cfg['train_batch_size'],
        drop_last=True
    ))
    has_train_files = os.path.exists(os.path.join(env.DIR_UNPACKED, filtered_train)) and \
                      len(list(jsonlines.open(os.path.join(env.DIR_UNPACKED, filtered_train)))) > 0
    if not has_train_files:
        raise RuntimeError("No train files provided")

    has_test_files = os.path.exists(os.path.join(env.DIR_UNPACKED, filtered_test)) \
                     and len(list(jsonlines.open(os.path.join(env.DIR_UNPACKED, filtered_test)))) > 0
    if has_test_files:
        test_ds = test_pipe(filtered_test, test_dataopts)
        test_ds = list(test_ds)
    else:
        traces.log("Warning: no test set provided, the number of files is zero")
        test_ds = None
    return train_ds, test_ds