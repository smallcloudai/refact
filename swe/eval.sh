#!/bin/bash
python /home/mitya/projects/aider-swe-bench/SWE-bench-docker/run_evaluation.py \
	--skip_existing --num_processes 8 \
    --swe_bench_tasks $(pwd)/princeton-nlp--SWE-bench_Lite.json \
    --log_dir $(pwd)/logs \
    --predictions_path $(pwd)/predictions/gpt35-gpt4/all_preds.jsonl