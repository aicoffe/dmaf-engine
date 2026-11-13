# Complexity Screening Worksheet

> Purpose: the working template for Phase 1. Collect requirements and constraints (5W1H8C), analyze complexity dimension by dimension with the screening method, and output the complexity list; conclusions go into the hld's "Architecture style and principles" section, the full process into the hld appendix.

## Part 1: Requirements and constraints collection (5W1H8C)

| Item | Content | Source / assumption note |
|---|---|---|
| Who (stakeholders) | | |
| When (timeline/milestones) | | |
| What (deliverables) | | |
| Where (deployment environment) | | |
| Why (problem to solve) | | |
| 1H (key business processes) | | |
| Performance | <scale requirement> | <demand or estimate> |
| Cost | <budget/server count cap> | |
| Time | <launch deadline> | |
| Reliability | <availability target> | |
| Security | <compliance/sensitive-data requirements> | |
| Compliance | <corporate standards> | |
| Technology | <team skill set/size> | |
| Compatibility | <existing system constraints> | |

Note: metrics from the business side among the 8C are **demands**, not design targets; point out and adjust the ones analysis shows are unreasonable.

## Part 2: Quantified estimation

```
daily volume ÷ 86400 = average per second
average per second × peak factor (≈3) = peak volume
peak × headroom factor (2–4: 4 when the base is low, 2 when high; >10 not recommended) = design target
```

| Metric | Daily volume | Average per second | Peak | Design target |
|---|---|---|---|---|
| Write TPS | | | | |
| Read QPS | | | | |
| Core table rows / daily growth | | | | |
| Online connections | | | | |

## Part 3: Six-dimension screening table

Answer for each dimension "is this dimension a primary complexity", with **evidence** (quantified derivation or requirement clause); gut feeling is not allowed.

| Dimension | Key question | Analysis and evidence | Primary complexity? | Priority |
|---|---|---|---|---|
| High performance | Does the target TPS/QPS exceed routine deployment capacity? Are there hard response-time requirements? | | Yes/No | |
| High availability | What is the availability target? Business cost of downtime? Cross-datacenter requirements? | | Yes/No | |
| Extensibility | Will business features keep changing? Expected team growth? | | Yes/No | |
| Low cost | Are there hard cost constraints (budget/machine count)? | | Yes/No | |
| Security | Sensitive data / compliance requirements / public-internet exposure? | | Yes/No | |
| Scale | Will user/data/business growth trigger an architectural qualitative change? | | Yes/No | |

## Part 4: Complexity list (deliverable)

1. Primary complexities (usually 1–2): <e.g., "highly available message storage (a lost message = a compliance incident)">
2. Secondary complexities: <...>
3. Priority ranking and rationale: <rank by business/technology/team combined>
4. Explicitly not solved this phase: <state which phase it is deferred to — evolution principle>

## Screening discipline

- Every dimension's conclusion must survive the question "why" — the evidence chain stays in this worksheet.
- All six dimensions being primary complexities = a misjudgment or a prior design error; you must re-converge and re-rank.
- A misidentification is a directional error: rather spend more time in Phase 1 than enter solution design carrying the wrong complexity.
