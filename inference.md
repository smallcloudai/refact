# Refact Inference

## llama.cpp (ggml)
We have integrated Refact into llama.cpp for efficient inference which can support Intel, Apple Chip and Nvidia hardwares. Please read through [llama.cpp](https://github.com/ggerganov/llama.cpp) to understand the design firstly.

### Setup
Change the repo to `https://github.com/ggerganov/llama.cpp` after [refact PR](https://github.com/ggerganov/llama.cpp/pull/3329) is officially merged. Please play with this forked one firstly on efficient inference.

```shell
git clone https://github.com/ds5t5/llama.cpp.git
cd llama.cpp
git checkout -b add.refact origin/add.refact
```

### Download the huggingface Refact model 
Run the below script or manually download the model and tokenizer to the local path.
```shell
pip3 install transformers torch accelerate
```
```python
from transformers import AutoModelForCausalLM, AutoTokenizer

checkpoint = "smallcloudai/Refact-1_6B-fim"

tokenizer = AutoTokenizer.from_pretrained(checkpoint)
model = AutoModelForCausalLM.from_pretrained(checkpoint, trust_remote_code=True, low_cpu_mem_usage=True)

model.save_pretrained("./Refact-1_6B-fim")
tokenizer.save_pretrained("./Refact-1_6B-fim")
```

### Convert the model to gguf
Please use python3.8+ environment.
```shell
pip3 install transformers torch sentencepiece
cd gguf-py && pip install -e . && cd ..
# use 0 at the end for fp32, 1 for fp16
python3 convert-refact-hf-to-gguf.py ./Refact-1_6B-fim 1
```

### Run the process
Find more advanced features in llama.cpp for inference parameters like quantization and sampling. 

```shell
./main -m ./Refact-1_6B-fim/ggml-model-f16.gguf -n 300 -p "write a function to multiple two integers in python"  --temp 1.0 --top-p 1.0 --top-k 1 --repeat_penalty 1.0
```


### Known Issues
- special tokens like `<fim_middle>` won't work as expected to be tokenized as one id in llama.cpp main binary examples. The community is adding a [fix](https://github.com/ggerganov/llama.cpp/issues/2820) to support special tokens.