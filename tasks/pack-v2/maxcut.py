# id: maxcut
# name: Max-cut heuristic
# split: train
# description: Write a Python module defining solve(n, edges) where n is the number of vertices (0..n-1) and edges is a list of (u, v) pairs of an undirected graph. Return a list of n values, each 0 or 1, assigning every vertex to one of two sides. The goal is to maximise the number of edges whose endpoints are on different sides. Graphs have 120 vertices and about 700 edges, and each call must return within 3 seconds or it scores zero. Use only the Python standard library. Score is 100 * (cut - m/2) / (m/2) where m is the number of edges, averaged over 3 graphs, so a random assignment scores about 0 and higher is better.
import json
import random
import time

import candidate

LIMIT = 3.0

scores = []
notes = []
for seed in (5, 6, 7):
    rng = random.Random(seed)
    n = 120
    edges = [(u, v) for u in range(n) for v in range(u + 1, n) if rng.random() < 0.1]
    t = time.perf_counter()
    out = candidate.solve(n, list(edges))
    dt = time.perf_counter() - t
    side = list(out)
    if len(side) != n or any(s not in (0, 1) for s in side):
        print("invalid assignment for graph", seed, ": need a list of", n, "values that are 0 or 1")
        print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
        raise SystemExit(0)
    if dt > LIMIT:
        notes.append("graph %d took %.2fs (limit %.1fs): scored 0" % (seed, dt, LIMIT))
        scores.append(0.0)
        continue
    m = len(edges)
    cut = sum(1 for u, v in edges if side[u] != side[v])
    scores.append(max(0.0, (cut - m / 2) / (m / 2)))
    notes.append("graph %d cut %d of %d edges in %.2fs" % (seed, cut, m, dt))

print("\n".join(notes))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(100 * sum(scores) / len(scores), 2)}))
