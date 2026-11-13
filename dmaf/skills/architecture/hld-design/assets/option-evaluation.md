# Alternatives and 360-Degree Evaluation Template

> Purpose: the working template for Phases 2/3. Alternatives are not mandatory: when a real architectural trade-off exists, design 2–3 markedly different alternatives (technology-selection level only), organize the 360-degree evaluation, and select by priority; design points with a best practice/de facto standard are adopted directly without alternatives (the evidence goes into the selection record). Write the alternative cards, evaluation table, and selection record into the corresponding ADR (`assets/adr-template.md`); they do not enter the hld body.

## Part 1: Alternative cards (one card per alternative)

```markdown
### Alternative N: <name> (e.g., the open-source Kafka option)

- Core idea: <one sentence capturing the essential difference from the other alternatives>
- Key design: <3–5 points, technology-selection level, no implementation details>
- Brief architecture diagram: <one sketch, optional>
```

Count and difference discipline (applies only to design points with real trade-offs):

- 2–3 alternatives: the cap is 3 — beyond that, effort does not allow it and the differences blur; straw-man alternatives padding the count are worse than none.
- Differences must be **architecture-level** (open source vs in-house, primary-standby vs cluster, MySQL vs NoSQL); parameter-level differences (detection interval 1 minute vs 5 minutes) do not make two alternatives.
- Do not confine yourself to the technologies you know; compose from mature patterns rather than chasing novelty for its own sake.

## Part 2: 360-degree evaluation table

When there are no alternatives (a best practice/de facto standard is adopted directly), skip Parts 1 and 2 and fill in the Part 3 selection record directly (state the evidence and the drawback mitigation). With alternatives, proceed as follows:

Evaluate every alternative dimension by dimension; write evaluations as "fact + degree" (e.g., "replication lag is in seconds; reading from a secondary has an N-second inconsistency window"), not adjectives.

| Quality attribute | Alternative 1 | Alternative 2 | Alternative 3 | Priority for this project |
|---|---|---|---|---|
| Performance (TPS/QPS/latency) | | | | <high/medium/low> |
| Availability | | | | |
| Hardware cost | | | | |
| Project investment (staffing/timeline) | | | | |
| Complexity (implementation/operations) | | | | |
| Security | | | | |
| Extensibility | | | | |
| Operability (fits the existing operations system?) | | | | |
| Team fit (skill set) | | | | |

The priority column is set **before evaluation** from business stage/team/constraints — it is the basis of the later selection, not numbers filled in afterwards.

## Part 3: Selection record

1. **Exclusion rationale**: per alternative, write "excluded because <first-priority attribute> is unmet / its risk is unacceptable" (e.g., Alternative 1 lacks operability — the operations system cannot cover it, so failures cannot be handled quickly). When there are no alternatives, this entry becomes the **adoption rationale**: the specific practice/standard entries cited and their applicability conditions ("everyone uses it" is not evidence).
2. **Selected alternative**: Alternative <N>.
3. **Drawback mitigation**: list all known drawbacks of the selected alternative and explain per item why each is acceptable (cite the three principles or quantitative evidence):
   - Drawback <performance short of the extreme> → <the current business scale does not need it and horizontal scaling can come later (evolution principle)>
   - Drawback <does not look advanced> → <the design goal is meeting business needs faster, not proving technical prowess (fitness principle)>
4. **Review meeting conclusion**: <time/participants (dev/test/ops/business) and how disagreements were resolved; the ADR is revised per the meeting conclusion>.

## Forbidden practices

- ❌ The count-comparison method: whoever has more pros wins — quality attributes do not all matter equally.
- ❌ The weighted-scoring method: weights and scores cannot be set objectively and degenerate into a numbers game.
- ❌ Rejecting an entire alternative over a single drawback; there is no perfect alternative — you compare overall fit.
- ❌ Using "industry-leading / the same as the big companies" as the selection rationale — that is a demand, not evidence.
