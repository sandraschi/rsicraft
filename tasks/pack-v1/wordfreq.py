# id: wordfreq
# name: Top-k word frequency
# split: heldout
# description: Write a Python module defining solve(text, k) that returns a list of the k most frequent words in text as (word, count) tuples. A word is a maximal run of ASCII letters a-z or A-Z; words are compared case-insensitively and returned in lower case. Order by count descending, ties broken by word ascending. If there are fewer than k distinct words return all of them. Use only the Python standard library. The score is the speedup over a naive implementation on a text of 20000 words, so make it as fast as possible.
import json
import random
import time

import candidate

LETTERS = set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")


def ref(text, k):
    words = []
    cur = []
    for ch in text:
        if ch in LETTERS:
            cur.append(ch.lower())
        elif cur:
            words.append("".join(cur))
            cur = []
    if cur:
        words.append("".join(cur))
    uniq = []
    for w in words:
        if w not in uniq:
            uniq.append(w)
    counted = [(w, words.count(w)) for w in uniq]
    counted.sort(key=lambda wc: (-wc[1], wc[0]))
    return counted[:k]


def best(f, args, k):
    b = 1e9
    for _ in range(k):
        t = time.perf_counter()
        f(*args)
        b = min(b, time.perf_counter() - t)
    return b


def norm(out):
    return [(str(w), int(c)) for w, c in out]


rng = random.Random(19)
vocab = ["".join(rng.choice("abcdefghij") for _ in range(rng.randint(1, 6))) for _ in range(300)]


def rand_text(n):
    seps = [" ", ", ", ". ", "\n", " - ", "1 ", "'"]
    parts = []
    for _ in range(n):
        w = rng.choice(vocab)
        if rng.random() < 0.2:
            w = w.capitalize()
        parts.append(w)
        parts.append(rng.choice(seps))
    return "".join(parts)


cases = [("", 3), ("Hello hello HELLO world", 1), ("a b c", 5), ("it's a dog-eat-dog world", 3), ("b a b a", 2), ("x1y2x", 2)]
cases += [(rand_text(rng.randint(0, 60)), rng.randint(0, 8)) for _ in range(30)]
wrong = []
for text, k in cases:
    try:
        ok = norm(candidate.solve(text, k)) == ref(text, k)
    except Exception as e:  # the evaluator reports it; the candidate must not crash
        ok = False
        print("exception on case", repr(text[:40]), k, "->", repr(e))
    if not ok:
        wrong.append((text[:40], k))
if wrong:
    print("wrong answers for cases (text prefix, k):", wrong[:3])
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)

args = (rand_text(20000), 10)
if norm(candidate.solve(*args)) != ref(*args):
    print("wrong answer on the timed input (20000 words, k=10)")
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)
t_ref = best(ref, args, 2)
t_cand = best(candidate.solve, args, 5)
print("ref_ms=%.3f cand_ms=%.4f" % (t_ref * 1000, t_cand * 1000))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(t_ref / max(t_cand, 1e-7), 2)}))
