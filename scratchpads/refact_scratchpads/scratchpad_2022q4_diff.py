import torch as th
import time
import termcolor
import collections

from refact_encoding import RefactEncoding, hlprint
import refact_code_contrast_2022q3
import refact_code_contrast_2022q3.contrast

from code_contrast.scratchpad.scratchpad import ScratchpadBase


from typing import Dict, Optional, Any, List, Tuple


class ScratchpadDiff(ScratchpadBase):
    def __init__(
        self,
        enc: RefactEncoding,
        intent: str,
        cursor_file: str,
        cursor0: int,
        cursor1: int,
        function: str,
        max_edits: int,
        sources: Dict[str, str],
        poi: Optional[List[Dict[str, Any]]] = None,
        **kwargs
    ):
        super().__init__(enc, **kwargs)
        self.intent = intent
        self.cursor_file = cursor_file
        self.cursor0 = cursor0
        self.cursor1 = cursor1
        self.function = function
        self.max_edits = max_edits
        self.sources = sources
        self.file_poi = collections.defaultdict(set)
        if poi is not None:
            # "poi": [{'filename': 'attractgame_mainloop.py', 'cursor0': 0, 'cursor1': 971, 'priority': 0.944}]
            for rec in poi:
                fn = rec.get("filename", None)
                if not isinstance(fn, str):
                    continue
                if fn.endswith(cursor_file):  # temporary kludge, remove
                    continue
                if not isinstance(rec.get("cursor0", None), int):
                    continue
                if not isinstance(rec.get("cursor1", None), int):
                    continue
                self.file_poi[fn].add(int(rec["cursor0"]))
                self.file_poi[fn].add(int(rec["cursor1"]))
        self.state_before_first_tpos = True
        self.diff: refact_code_contrast_2022q3.ContrastDiff = None
        self.diff_out: Optional[refact_code_contrast_2022q3.ContrastDiff] = None
        self.diff_out_us: Optional[refact_code_contrast_2022q3.UntokenizeState] = None
        self.highlight = []
        self.highlight16 = []
        self.t_cursor0 = -1
        self.t_cursor1 = -1
        self.tpos_cursor1 = -1
        self.edits_uploaded = 0
        self.prompt_edits = 0
        self.cursorfile_tokens1 = None
        self.cursorfile_tokens2 = None
        self.cursorfile_map1to2 = None
        self.cursorfile_map2to1 = None
        self.increase_logits = []
        self.no_stop_tokens_until = -1
        self.selected_newlines = -1
        self.selection_is_important = (self.function in ["diff-atcursor", "diff-selection"])
        self.backward_cache_snippet: str = ""
        self.backward_cache_tokens: List[int] = []
        self.backward_cache_cursor: int = 0
        self.ugly_hack_reattach_next_line: Optional[str] = None
        self.P1 = 0.35
        self.P2 = 0.20
        self.JP1 = 0.20

    def set_model_thresholds(self, P1, P2, JP1, **more):
        self.P1 = P1
        self.P2 = P2
        self.JP1 = JP1
        super().set_model_thresholds(**more)

    def before_token_selection(
            self,
            m: Any,
            b: int,
            logits: th.Tensor,
            heads: List[th.Tensor],
            **unused
    ) -> Dict[str, Any]:
        if self.state_before_first_tpos:
            if self.function == "highlight":
                self.highlight_method4(m, b, logits, heads)
            self.state_before_first_tpos = False
        prev_token = self.diff.r[-1]
        suggest_tokens = []
        logits_intrusion: Dict[int, float] = dict()
        if prev_token == self.enc.CHUNK:
            for tpos in self.increase_logits:
                logits_intrusion[tpos] = +4.5
        if (
                self.diff_out_us is not None and
                self.diff_out_us.state == refact_code_contrast_2022q3.contrast.DEL and
                self.diff_out_us.brewing_edit.real_delstart != -1 and
                self.diff_out_us.brewing_edit.fn == self.cursor_file
        ):
            e = self.diff_out_us.brewing_edit
            scratch = self.diff_out_us.scratch[e.fn]
            if self.cursorfile_tokens2 is not None:
                tokens2 = self.cursorfile_tokens2
                assert all(tokens2[i] == scratch[i] for i in range(len(scratch)))
                # print("todel:", termcolor.colored(self.enc.decode(scratch[e.real_delstart:e.real_delends]), "yellow"))
                # good, _ = self._lookahead_ignoring_tpos(scratch, e.real_delstart, e.todel)
                # print("suggest: [%s]" % termcolor.colored(self.enc.decode(scratch[e.real_delends:e.real_delends + 8]).replace("\n", "\\n"), "blue"))
                suggest_tokens = scratch[e.real_delends:e.real_delends + 10]
                suggest_tokens = [t for t in suggest_tokens if not self.enc.is_tpos(t)][:8]
                if self.selection_is_important:
                    beyond_selection = self.diff_out_us.brewing_edit.real_delends - self.t_cursor1
                    if beyond_selection >= -1:
                        extra_newlines = len([t for t in scratch[self.t_cursor1:self.diff_out_us.brewing_edit.real_delends] if t == self.enc.LF])
                        if extra_newlines >= 0:
                            logits_intrusion[self.enc.ESCAPE] = 5.0 + 0.5 * extra_newlines
                # edit works like this: scratch[e.real_delstart:e.real_delends] = e.toins
        T = logits.shape[0]
        UPTO = 10
        if T > UPTO and self.backward_cache_cursor > UPTO+1:  # infill function only
            # self.backward_cache_cursor -- position of the infill token minus about three
            # self.backward_cache_snippet -- original file until cursor
            argmax = th.argmax(logits[self.backward_cache_cursor-UPTO-1:self.backward_cache_cursor], dim=1)
            self.backward_cache_tokens = []
            for minus in range(1, UPTO):
                t = self.diff.r[self.backward_cache_cursor - minus]
                if self.enc.is_tpos(t):
                    continue
                if t == argmax[-minus-1]:
                    self.backward_cache_tokens = [t] + self.backward_cache_tokens
                else:
                    self.debuglog("PROMPT TOKEN \"%s\" BUT ARGMAX FROM PREVIOUS \"%s\""  % (self.enc.decode([t]), self.enc.decode([argmax[-minus-1].item()])))
                    break
            self.debuglog("BACK CACHE \"%s\"" % (self.enc.decode(self.backward_cache_tokens).replace("\n", "\\n")))
        return dict(
            logits_intrusion=logits_intrusion,
            suggest_tokens=suggest_tokens,
        )

    def after_token_selection(
            self,
            m,
            chosen_token: th.Tensor,
            **unused
    ) -> Dict[str, Any]:
        t = chosen_token.item()
        self.diff.r.append(t)
        self.diff_out_catch_up()
        self.generated_tokens_n += 1
        if self.generated_tokens_n <= 3:
            self.backward_cache_tokens.append(t)
        return dict()

    def toplevel_fields(self):
        if self.function == "highlight":
            return {"highlight_tokens": self.highlight, "highlight_lines": self.highlight16}
        if self.function == "infill":
            cached_longer_txt = self.enc.decode(self.backward_cache_tokens)
            #self.debuglog("backward_cache \"%s\"" % (cached_longer_txt.replace("\n", "\\n")))
            #self.debuglog("backward_cache_snippet \"%s\"" % (self.backward_cache_snippet.replace("\n", "\\n")))
            for cut_extra in range(min(len(cached_longer_txt), 40)):
                cached = cached_longer_txt[:len(cached_longer_txt)-cut_extra]
                if self.backward_cache_snippet.endswith(cached):
                    # self.debuglog("backward_cache final \"%s\"" % (cached.replace("\n", "\\n")))
                    return {"backward_cache": cached}
            # self.debuglog("backward_cache 🤷")
            return {"backward_cache": ""}
        return {}

    def completion(self, final: bool):
        if final:
            if self.diff_out_us is not None:
                self.diff_out_catch_up()
                self.diff_out.untokenize_finish_state(self.diff_out_us, self.diff_out_cursor)
            else:
                self.debuglog("ScratchpadDiff: nothing useful available")
                return None
        else:
            self.diff_out_catch_up()
        dest_tokens = self.diff_out.apply_edits_return_dest(self.diff_out_us)
        result = {}
        for fn in dest_tokens:
            result[fn] = self.enc.decode(self.diff_out.dest_tokens[fn])
        self.debuglog("ScratchpadDiff: final=%i" % final, self.diff_out_us.stats, self.finish_reason)
        return result

    def diff_out_catch_up(self):
        if self.diff_out_us is None:
            return
        def finish(reason):
            self.finish_reason = reason
            self.diff_out.untokenize_finish_state(self.diff_out_us, self.diff_out_cursor)
            if reason in ["ins-stoptoken", "ins-stop-lf", "ins-stop-lflf"] and self.ugly_hack_reattach_next_line is not None:
                if len(self.diff_out.edits) == 1:
                    self.debuglog("REATTACH '%s'\n" % (self.ugly_hack_reattach_next_line.replace("\n", "\\n")))
                    self.diff_out.edits[0].toins.extend(self.enc.encode(self.ugly_hack_reattach_next_line) + [self.enc.LF])
            self.diff_out_us.state = refact_code_contrast_2022q3.contrast.WAIT
        try:
            while self.diff_out_cursor < len(self.diff.r):
                c = self.diff_out_cursor
                t = self.diff.r[c]
                if t==self.enc.EOT:
                    finish("eot")
                    break
                self.diff_out.untokenize_new_token(self.diff_out_us, t, c)
                self.diff_out_cursor += 1
                if self.diff_out_us.state == refact_code_contrast_2022q3.contrast.CHUNK and self.max_edits >= 0 and len(self.diff_out.edits) - self.prompt_edits >= self.max_edits:
                    finish("max-edits")
                    break
                if c >= self.no_stop_tokens_until and self.diff_out_us.state == refact_code_contrast_2022q3.contrast.INS:
                    if t in self.stop_tokens:
                        finish("ins-stoptoken")
                        break
                    if self.stop_lf_lf and (self.diff.r[c - 1], t) == (self.enc.LF, self.enc.LF):
                        finish("ins-stop-lflf")
                        break
                    if self.stop_lf and t == self.enc.LF:
                        finish("ins-stop-lf")
                        break
                    if t == self.enc.LF:
                        if self.stream:
                            self.needs_upload = True
                if self.diff_out_us.state in [refact_code_contrast_2022q3.contrast.DEL, refact_code_contrast_2022q3.contrast.SHIFT]:
                    # print("TEST epos=%i in %s\n\n" % (self.diff_out_us.e_tpos, self.increase_logits))
                    if len(self.increase_logits) > 0 and (self.diff_out_us.brewing_edit.tpos not in self.increase_logits):
                        finish("out-of-selection")
                        break
        except refact_code_contrast_2022q3.DecodeError as e:
            self.debuglog("Exception in diff_out.untokenize_new_token: %s" % e)
            self.finish_reason = "diff-application-error"

    def prompt_infill(self, T):
        for fn, text in self.sources.items():
            if self.cursor_file == fn:
                cut_slash_n = text[self.cursor0:]
                slash_n_idx = cut_slash_n.find("\n")
                if slash_n_idx >= 0:
                    cut_slash_n = cut_slash_n[slash_n_idx+1:]
                if 1:
                    # To fully remove this, we need to retrain the model, the q1 version should fix it
                    next_slash_n = cut_slash_n.find("\n")
                    if next_slash_n >= 0:
                        self.ugly_hack_reattach_next_line = cut_slash_n[:next_slash_n]
                        self.debuglog("self.ugly_hack_reattach_next_line \"%s\"" % self.ugly_hack_reattach_next_line.replace("\n", "\\n"))
                    else:
                        self.ugly_hack_reattach_next_line = None
                self.odm["orig"][fn] = text[:self.cursor0] + self.enc.decode([self.enc.INFILL]) + cut_slash_n
                self.odm["dest"][fn] = text[:self.cursor0] + self.enc.decode([self.enc.DUMMY]) + cut_slash_n
            elif fn in self.file_poi:
                self.odm["orig"][fn] = text
                self.odm["dest"][fn] = text
        self.orig_tokens = self.diff.from_odm_dict(
            self.odm,
            n_ctx=(T - self.max_tokens),
            tight_shrink=True,
            exact_cx_lines0=2,
            exact_cx_lines1=0,
            external_poi=self.file_poi,
            )
        self.backward_cache_cursor = self.diff.r.index(self.enc.INFILL)
        self.diff.write_edits()
        assert len(self.diff.edits) == 1
        while len(self.diff.r) > 0:
            t = self.diff.r.pop()
            if t == self.enc.DUMMY:
                break
        del3more = 3
        self.backward_cache_snippet = text[:self.cursor0]
        self.backward_cache_deltokens = 0
        while len(self.diff.r) > 0 and self.diff.r[-1] not in [self.enc.LF] and del3more > 0:
            self.diff.r.pop()
            self.backward_cache_cursor -= 1
            del3more -= 1

    def prompt_edit_chain(self, T):
        minrev = 10000
        for fn, text in self.sources.items():
            fn, revision = refact_code_contrast_2022q3.contrast.parse_fn(fn)
            if revision is None:
                continue
            if self.function != "edit-chain":
                continue
            if self.cursor_file != fn:
                continue
            if revision < minrev:
                minrev = revision
            else:
                continue
            self.odm["orig"][fn] = text
            # self.debuglog("revision", revision)
            # self.debuglog("EDIT CHAIN BASE", text)
        for fn, text in self.sources.items():
            fn, suffix = refact_code_contrast_2022q3.contrast.parse_fn(fn)
            if suffix is not None:
                continue
            self.odm["dest"][fn] = text
            # self.debuglog("EDIT CHAIN DEST", text)
        self.orig_tokens = self.diff.from_odm_dict(
            self.odm,
            n_ctx=(T - self.max_tokens),
            tight_shrink=True,
            exact_cx_lines0=2,
            exact_cx_lines1=0,
            )
        self.diff.write_edits()
        assert self.diff.r[-1] == self.enc.EOT
        self.diff.r = self.diff.r[:-1]
        self.prompt_edits = len(self.diff.edits)

    def prompt_normal_diff(self, T):
        # Highlight also goes here
        for fn, text in self.sources.items():
            self.odm["orig"][fn] = text
            if self.cursor_file == fn:
                # make sure cursor01 is visible
                self.odm["dest"][fn] = text[:self.cursor0] + self.enc.decode([self.enc.DUMMY]) + text[self.cursor1:]
            else:
                self.odm["dest"][fn] = text
        self.orig_tokens = self.diff.from_odm_dict(
            self.odm,
            n_ctx=(T - self.max_tokens),
            tight_shrink=True,
            exact_cx_lines0=2,
            exact_cx_lines1=0,
            )
        self.diff_out = refact_code_contrast_2022q3.contrast.ContrastDiff(self.enc)
        self.diff_out_us = self.diff_out.untokenize_init(self.orig_tokens)
        self.diff_out_cursor = 0
        if self.cursor0 != -1:
            self._find_selection_in_tokens()
        if self.function == "highlight":
            self.diff.write_esc_chunk()
            return
        if self.selected_newlines in [0, 1] and self.selection_is_important:
            # selected single line or atcursor, write most of the chunk immediately
            self.max_edits = 1
            # tpos = self.cursorfile_tokens2[self.tpos_cursor0]
            # assert self.enc.is_tpos(tpos)
            # self.diff.r.append(tpos)
            self.diff.write_edits()
            assert len(self.diff.edits) == 1
            while len(self.diff.r) > 0:
                t = self.diff.r.pop()
                if t == self.enc.DUMMY:
                    break
            while len(self.diff.r) > 0 and self.diff.r[-1] not in [self.enc.LF]:
                self.diff.r.pop()
        elif self.cursorfile_tokens2 is not None and self.selection_is_important:
            # multi line selection, logits
            i = self.t_cursor0
            over = False
            while 1:
                t = self.cursorfile_tokens2[i]
                if self.enc.is_tpos(t):
                    self.debuglog("diff-selection increase logits", hlprint(self.enc, [t]))
                    self.increase_logits.append(t)
                    if over: break
                if i >= self.t_cursor1:
                    if len(self.increase_logits) > 0:
                        break
                    over = True
                if i >= len(self.cursorfile_tokens2):
                    break
                i += 1
            self.increase_logits.append(self.enc.EOT)
            self.diff.write_esc_chunk()
        else:
            self.diff.write_esc_chunk()

    def prompt(self, T):
        t0 = time.time()
        self.diff = refact_code_contrast_2022q3.contrast.ContrastDiff(self.enc)
        self.odm = {
            "orig": dict(),
            "commitmsg": self.intent,
            "dest": dict(),
        }
        # "^(highlight|infill|diff-anywhere|diff-atcursor|diff-selection|edit-chain)$"
        if self.function == "infill":
            self.prompt_infill(T)
        elif self.function == "edit-chain":
            self.prompt_edit_chain(T)
        else:
            self.prompt_normal_diff(T)
        if len(self.diff.r) >= T:
            self.debuglog("PACKING FAILED %i TOKENS\n" % (len(self.diff.r)))
            return []
        self.no_stop_tokens_until = len(self.diff.r)
        if self.diff_out is None:
            self.diff_out = refact_code_contrast_2022q3.contrast.ContrastDiff(self.enc)
            self.diff_out_us = self.diff_out.untokenize_init(self.orig_tokens)
            self.diff_out_cursor = 0
            self.diff_out_catch_up()
            if self.cursor0 != -1:
                self._find_selection_in_tokens()
        return self.diff.r

    def _find_selection_in_tokens(self):
        assert self.cursor0 > -1 and self.cursor1 > -1, "cursor not set cursor0=%i cursor1=%i" % (self.cursor0, self.cursor1)
        if self.cursorfile_tokens1 is None:
            self.diff_out_catch_up()
            # equals to self.diff_out
            self.cursorfile_tokens1 = \
                self.diff.orig_tokens[self.cursor_file]
            # all tokens including previously cut top/bottom, with postion tokens in the middle
            self.cursorfile_tokens2 = \
                self.diff_out.orig_withpos[self.cursor_file]
            # works fast ~1ms
            self.cursorfile_map1to2, self.cursorfile_map2to1 = \
                self._fn_create_map1to2(self.cursorfile_tokens1, self.cursorfile_tokens2)

        # potentially slow if we get non-unicode encoded string
        self.t_cursor0, self.t_cursor1 = self._find_cursor_in_tokens(
            self.cursor0, self.cursor1, self.cursorfile_tokens1, self.cursorfile_map1to2)

        # self.debuglog(
        #     termcolor.colored(self.enc.decode(self.cursorfile_tokens2[:self.t_cursor0]), "yellow") +
        #     termcolor.colored("|", "green") +
        #     termcolor.colored(self.enc.decode(self.cursorfile_tokens2[self.t_cursor0:self.t_cursor1]), "red") +
        #     termcolor.colored("|", "green") +
        #     termcolor.colored(self.enc.decode(self.cursorfile_tokens2[self.t_cursor1:]), "yellow")
        #     )
        self.selected_newlines = len([t for t in self.cursorfile_tokens2[self.t_cursor0:self.t_cursor1] if t == self.enc.LF])

    def _fn_create_map1to2(self, tokens1: List[int], tokens2: List[int]) -> Tuple[List[int], List[int]]:
        i1 = 0
        map1to2 = []
        map2to1 = []
        # At the end after escape, only diamonds and the last tpos are allowed:
        seen_escape = False
        for i2, t in enumerate(tokens2):
            map2to1.append(i1)
            if self.enc.is_tpos(t):
                continue
            if i1 == len(map1to2) and i1 < len(tokens1):
                map1to2.append(i2)
            if t == self.enc.ESCAPE:
                seen_escape = True
                i1 += 1
            elif t == self.enc.DIAMOND:
                i1 += 1
            elif not seen_escape:
                assert t == tokens1[i1]
                i1 += 1
            else:
                assert 0
        assert len(tokens1) == len(map1to2)
        assert len(tokens2) == len(map2to1)
        return map1to2, map2to1

    def _find_cursor_in_tokens(self, cursor0: int, cursor1: int, tokens: List[int], map1to2: List[int]):
        assert cursor0 <= cursor1
        t_cursor0 = None
        t_cursor1 = None
        token_idx = 0
        text_idx = 0

        def decodable_text_steps(token_idx: int) -> Tuple[int, int]:
            token_jdx = token_idx + 1
            while token_jdx <= len(tokens):
                try:
                    text = self.enc.decode_utf8(tokens[token_idx:token_jdx])
                    return token_jdx - token_idx, len(text)
                except UnicodeDecodeError:
                    token_jdx += 1
            else:
                self.debuglog("invalid tokens: cannot decode utf8 sequence")
                return len(tokens) - token_idx, len(self.enc.decode(tokens[token_idx:]))

        while token_idx < len(tokens):
            token_step, text_step = decodable_text_steps(token_idx)
            if t_cursor0 is None and text_idx + text_step > cursor0:
                t_cursor0 = token_idx
            if t_cursor1 is None and text_idx >= cursor1:
                t_cursor1 = token_idx
            token_idx += token_step
            text_idx += text_step

        t_cursor0 = len(tokens) - 1 if t_cursor0 is None else t_cursor0
        t_cursor1 = t_cursor1 if cursor0 < cursor1 else t_cursor0  # special case
        t_cursor1 = len(tokens) - 1 if t_cursor1 is None else t_cursor1
        assert t_cursor0 <= t_cursor1
        return map1to2[t_cursor0], map1to2[t_cursor1]

    def highlight_method4(self, m: Any, b, logit, heads):
        t0 = time.time()
        x_bte = heads["x_bte"][b:b+1]
        first_bt = th.zeros_like(x_bte[:, :, 0])
        first_bt[:, 0] = 1
        diffhlpoint_bt = th.zeros_like(x_bte[:, :, 0])
        diffhlpoint_bt[:, -1] = 1
        # ed_joint = m.highlight_forward(x_bte, first_bt, diffhlpoint_bt)
        # e2_logits = m.bidir_2logits(ed_joint)
        inside = m.highlight_forward(x_bte, first_bt, diffhlpoint_bt)
        ed_joint = x_bte + inside
        e2_logits = m.bidir_2logits(m.bidir_2logits_ln(ed_joint))

        pd_hl = th.distributions.categorical.Categorical(logits=e2_logits[0] / 1.0)
        probs = pd_hl.probs
        t1 = time.time()
        tokens1, tokens2, map2to1 = self.cursorfile_tokens1, self.cursorfile_tokens2, self.cursorfile_map2to1
        # tokens1 without position tokens
        # tokens2 with position tokens
        # both tokens1 and tokens2 are full, not cut at top/bottom
        start = self.diff.fn2tstart[self.cursor_file]   # index in r
        end = start + len(self.diff.fn2tind[self.cursor_file])
        cut0 = self.diff_out.fn2cut0[self.cursor_file]
        self.highlight = []
        self.highlight16 = []
        inside_yellow = False
        inside_purple = False
        starts16 = -1
        ends16 = -1
        def no_longer_16():
            nonlocal starts16, ends16
            if starts16 == -1:
                return
            tmp1 = self.enc.decode(tokens1[:starts16])
            tmp2 = self.enc.decode(tokens1[:ends16])
            self.highlight16.append((len(tmp1), len(tmp2), 0.15))
            starts16 = -1
            ends16 = -1
        for ti in range(start, end):
            if self.diff.r[ti] == self.enc.ESCAPE:
                break
            assert tokens2[ti - start + cut0] == self.diff.r[ti]
            if self.diff.r[ti] != self.enc.LF:
                continue
            prev_lf = ti - 1
            while prev_lf >= start and self.diff.r[prev_lf] != self.enc.LF:
                prev_lf -= 1
            p0 = float(probs[ti][0].item())
            p1 = float(probs[ti][1].item())
            p2 = float(probs[ti][2].item())
            tokens0pos = map2to1[prev_lf + 1 - start + cut0]
            tokens1pos = map2to1[ti - start + cut0]
            want16 = 0
            if p1 > self.P1:
                want16 = 0.3
                inside_yellow = True
            elif p2 > self.P2 and inside_yellow:
                want16 = 0.15
                inside_purple = True
            else:
                inside_yellow = False
                inside_purple = False
                no_longer_16()
            if want16 > 0:
                if starts16 == -1:
                    starts16 = tokens0pos
                ends16 = tokens1pos
            self.debuglog(
                "%-60s" % (self.enc.decode(self.diff.r[prev_lf+1:ti+1]).replace("\n", "\\n")),
                termcolor.colored(
                    " %0.1f%% %0.1f%% %0.1f%%" % (100*p0, 100*p1, 100*p2),
                    ("magenta" if inside_purple else "red") if inside_yellow else None,
                ),
            )
            if inside_yellow:
                for tj in range(prev_lf+1, ti+1):
                    jp0 = float(probs[tj][0].item())
                    jp1 = float(probs[tj][1].item())
                    jp2 = float(probs[tj][2].item())
                    self.debuglog(
                        termcolor.colored(
                            "  %-20s" % (self.enc.decode(self.diff.r[tj:tj+1]).replace("\n", "\\n")),
                            "blue"),
                        termcolor.colored(
                            " %0.1f%%" % (100*jp1,),
                            "yellow" if (jp1 > self.JP1) else None,
                        )
                    )
                    if self.enc.is_tpos(self.diff.r[tj]):
                        continue
                    if jp1 > self.JP1:
                        tokens1pos = map2to1[tj - start + cut0]
                        tokens2pos = map2to1[tj + 1 - start + cut0]
                        jtmp1 = self.enc.decode(tokens1[:tokens1pos])
                        jtmp2 = self.enc.decode(tokens1[:tokens2pos])
                        self.highlight.append((len(jtmp1), len(jtmp2), 0.95))
                        # self.highlight.extend(self.enc.decode(tokens1[:tokens1pos]))
        no_longer_16()
        t2 = time.time()
        self.debuglog("highlight_method4 calc %0.2fs tokens %0.2fs" % (t1-t0, t2-t1))

    def dump(self) -> bytes:
        import pickle
        enc = self.enc
        self.enc = None
        if self.diff is not None:
            self.diff.enc = None
        if self.diff_out is not None:
            self.diff_out.enc = None
        d = pickle.dumps(self)
        self.enc = enc
        if self.diff is not None:
            self.diff.enc = enc
        if self.diff_out is not None:
            self.diff_out.enc = enc
        return d

    def set_enc(self, enc: RefactEncoding):
        self.enc = enc
        if self.diff is not None:
            self.diff.enc = enc
        if self.diff_out is not None:
            self.diff_out.enc = enc