# {'temperature': 0.3, 'top_p': 1.0, 'top_n': 0, 'max_tokens': 500, 'created': 1682669743.180327, 'stop_tokens': [], 'id': 'uchat-Gv1CSwCGXEM1-CSZJlmLgIVTu', 'object': 'chat_completion_req',
# 'account': 'oleg@smallcloud.tech',
# 'model': 'bigcode/15b',
# 'messages': [{'role': 'user', 'content': "import pygame\nimport random\n\n\nclass Game:\n    def __init__(self):\n        pygame.init()\n        self.screen = pygame.display.set_mode((800, 600))\n        self.clock = pygame.time.Clock()\n        self.running = True\n        self.circles = []\n        self.colors = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0), (0, 255, 255), (255, 0, 255)]\n        self.color_index = 0\n        self.score = 0\n        self.font = pygame.font.SysFont('Arial', 24)\n        self.text = self.font.render('Score: ' + str(self.score), True, (255, 255, 255))\n        self.text_rect = self.text.get_rect()\n        self.text_rect.center = (400, 300)\n\n    def run(self):\n        while self.running:\n            self.clock.tick(60)\n            self.events()\n            self.updat\n            self.draw()\n\n    def update(self):\n        for circle in self.circles:\n            circle.update()\n\n    def draw(self):\n        self.screen.fill((0, 0, 0))\n        for circle in self.circles:\n            circle.draw(self.screen)\n        self.screen.blit(self.text, self.text_rect)\n        pygame.display.flip()\n\n    def events(self):\n        for event in pygame.event.get():\n            if event.type == pygame.QUIT:\n                self.running = False\n\n    def init_circles(self):\n        self.circles = []\n        for i in range(10):\n            self.circles.append(Circle(self, self.colors[self.color_index]))\n            self.color_index = (self.color_index + 1) % len(self.colors)\n        self.color_index = 0\n\n\nclass Circle:\n    def __init__(self, game, color):\n        self.game = game\n        self.color = color\n        self.radius = random.randrange(10, 30)\n        self.x = random.randrange(self.radius, self.game.screen.get_width() - self.radius)\n        self.y = random.randrange(self.radius, self.game.screen.get_height() - self.radius)\n        self.x_speed = random.randrange(-2, 2)\n        self.y_speed = random.randrange(-2, 2)\n\n    def update(self):\n        self.x += self.x_speed"}, {'role': 'assistant', 'content': "Thanks for context, what's your question?"}, {'role': 'user', 'content': 'aaa'}, {'role': 'assistant', 'content': ''}, {'role': 'user', 'content': 'aaa'}],
# 'stream': True}

import torch as th

from typing import List, Any, Dict, Set, Optional, Union, Tuple

from code_contrast.scratchpad.scratchpad import ScratchpadBase
from code_contrast.encoding.smc_encoding import SMCEncoding
from code_contrast.scratchpad.bigcode_chat_prompt import base_msg


class ScratchpadBigChat(ScratchpadBase):
    def __init__(
            self,
            enc: SMCEncoding,
            messages: List[Dict[str, str]],
            **kwargs
    ):
        super().__init__(enc, **kwargs)
        for k, v in kwargs.items():
            self.debuglog("call %s = %s" % (k, v))
        self.enc = enc
        self.messages = messages
        self._prompt = ""
        self._completion = []
        self._completion_txt = ""
        self._tokens = []
        self._tokens_produced = 0
        self._upload_moratorium = 5

    def before_token_selection(self, m, **unused) -> Dict[str, Any]:
        return dict()

    def after_token_selection(
            self,
            m,
            chosen_token: th.Tensor,
            **unused
    ) -> Dict[str, Any]:
        self._tokens_produced += 1
        if self._upload_moratorium > 0:
            self._upload_moratorium -= 1
        if self._tokens_produced % 3 == 0 and self._upload_moratorium == 0:
            self.needs_upload = True
        # print("self._tokens_produced", self._tokens_produced, "self._upload_moratorium", self._upload_moratorium)
        t = chosen_token.item()
        self._tokens.append(t)
        if chosen_token == self.enc.EOT:
            self.finish_reason = "eot"
        if not self.finish_reason:
            self._completion.append(t)
        if chosen_token in self.stop_tokens:
            self.finish_reason = "stoptoken"

        if not self.finish_reason:
            self._completion_txt = self.enc.decode(self._completion)
            if '\n\nHuman: ' in self._completion_txt:
                self.finish_reason = "chat-stop-seq"
                self._completion_txt = self._completion_txt.split("\n\nHuman: ")[0].rstrip()
            if "\n\n-----" in self._completion_txt:
                self.finish_reason = "chat-stop-seq"
                self._completion_txt = self._completion_txt.split("\n\n-----")[0].rstrip()

        t_str = self.enc.decode([t])
        if "\n" in t_str:
            self._upload_moratorium = 3  # might be "Human:", don't want this partially uploaded
        if self.stop_lf and t_str.startswith("\n"):
            self.finish_reason = "stop-lf"
        if self.stop_lf_lf and t_str.startswith("\n\n"):
            self.finish_reason = "stop-lflf"
        return dict()

    def prompt(self, T: int):
        # Human: Write a function that takes two lists and returns a list that has alternating elements from each input list.
        # Assistant: Sure. Here is a function that does that.
        p = base_msg
        for msgdict in self.messages:
            p = p.rstrip()
            p += "\n\n"
            if msgdict["role"] == "user":
                p += "Human: " + msgdict["content"]
            else:
                p += "Assistant: " + msgdict["content"]
        p += "\n\nAssistant:"
        self._prompt = p
        self._tokens = self.enc.encode(p)
        self.debuglog("prompt %i chars -> %i tokens" % (len(p), len(self._tokens)))
        self._completion.clear()
        return self._tokens

    def completion(self, final: bool):
        result = {}
        result["chat__role"] = "assistant"
        result["chat__content"] = self._completion_txt
        return result
