# id: schedule
# name: Weighted tardiness scheduling
# split: heldout
# description: Write a Python module defining solve(jobs) where jobs is a list of (processing_time, due_date, weight) integer tuples. All jobs run one after another on a single machine starting at time 0, with no idle time. Return the processing order as a list containing every job index exactly once. A job that finishes at time C costs weight * max(0, C - due_date); the goal is to minimise the total cost. There are 60 jobs per instance and each call must return within 3 seconds or it scores zero. Use only the Python standard library. Score is 100 times the cost of the earliest-due-date order divided by the cost of your order, averaged over 3 instances, so 100 means "as good as earliest due date" and higher is better.
import json
import random
import time

import candidate

LIMIT = 3.0


def cost(jobs, order):
    t = 0
    c = 0
    for j in order:
        p, d, w = jobs[j]
        t += p
        if t > d:
            c += w * (t - d)
    return c


ratios = []
notes = []
for seed in (31, 32, 33):
    rng = random.Random(seed)
    jobs = [(rng.randint(1, 20), 0, rng.randint(1, 10)) for _ in range(60)]
    total = sum(p for p, _, _ in jobs)
    jobs = [(p, rng.randint(int(0.1 * total), int(0.6 * total)), w) for p, _, w in jobs]
    t = time.perf_counter()
    out = candidate.solve(list(jobs))
    dt = time.perf_counter() - t
    order = list(out)
    if sorted(order) != list(range(len(jobs))):
        print("invalid order for instance", seed, ": not a permutation of all job indices")
        print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
        raise SystemExit(0)
    if dt > LIMIT:
        notes.append("instance %d took %.2fs (limit %.1fs): scored 0" % (seed, dt, LIMIT))
        ratios.append(0.0)
        continue
    edd = sorted(range(len(jobs)), key=lambda j: jobs[j][1])
    ratios.append(cost(jobs, edd) / max(cost(jobs, order), 1))
    notes.append("instance %d cost %d (EDD %d) in %.2fs" % (seed, cost(jobs, order), cost(jobs, edd), dt))

print("\n".join(notes))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(100 * sum(ratios) / len(ratios), 2)}))
