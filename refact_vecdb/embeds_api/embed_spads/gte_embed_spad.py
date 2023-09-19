import os

from typing import Union, List

import torch.nn.functional as F
from torch import Tensor
from transformers import AutoTokenizer, AutoModel


__all__ = ['GTEEmbeddingSpad']
os.environ['TOKENIZERS_PARALLELISM'] = 'false'


def average_pool(
        last_hidden_states: Tensor,
        attention_mask: Tensor
) -> Tensor:
    last_hidden = last_hidden_states.masked_fill(~attention_mask[..., None].bool(), 0.0)
    return last_hidden.sum(dim=1) / attention_mask.sum(dim=1)[..., None]


class GTEEmbeddingSpad:
    def __init__(
            self,
            model_name: str = 'thenlper/gte-base'
    ):
        self._model_name = model_name
        self._model = AutoModel.from_pretrained(self._model_name)
        self._tokenizer = AutoTokenizer.from_pretrained(self._model_name)

    def create(self, text: Union[str, List[str]]) -> List[List[float]]:
        text = text if isinstance(text, list) else [text]

        batch_dict = self._tokenizer(
            text,
            max_length=512,
            padding=True,
            truncation=True,
            return_tensors='pt'
        )

        outputs = self._model(**batch_dict)
        embeddings = average_pool(outputs.last_hidden_state, batch_dict['attention_mask'])

        embeddings = F.normalize(embeddings, p=2, dim=1)
        return embeddings.tolist()

    def count_tokens(self, text: str) -> int:
        batch_dict = self._tokenizer(
            text,
            max_length=32_000,
            padding=False,
            truncation=False,
            return_tensors='pt'
        )
        return batch_dict['input_ids'].shape[1]


if __name__ == '__main__':
    gte = GTEEmbeddingSpad()
    import IPython; IPython.embed(); quit()
