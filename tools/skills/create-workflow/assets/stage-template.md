---
id: <slug>                        # = file-name stem, English kebab-case
phase: <phase>                    # = enclosing directory name
collaboration: single             # single (exactly 1 agent) | sequential | parallel (≥2)
agents:
  - agent: <agent-name>           # asset name from the project's agents/; if missing, flag "asset to be created"
    skills: []                    # asset names from the project's skills/
    inputs: ["dmaf-output/{SESSION_ID}/<upstream dir>/{topic}-<upstream artifact>.md"]   # full path templates; external pre-existing files become soft inputs {artifact: …, required: false}
    outputs: ["dmaf-output/{SESSION_ID}/<this stage's dir>/{topic}-<this stage's output>.md"]  # promise only artifacts that are necessarily produced
    task_description: "<Based on '{topic}'…, produce… (output-format conventions go into the skill's execution instructions)>"   # in the task statement's language, supports {variables}; the raw requirement text enters only via "{TASK}" in the first stage
retry:
  max_attempts: 1                 # retry cap after the first attempt (1 = 2 attempts in total)
  on_exhausted: abort_workflow    # abort_workflow | continue
approval:
  required: false                 # human review → true + gate (the revision loop is built in here; keep it out of the structure)
  gate: ""
execution: always                 # always | conditional
# on_skip: skip                   # optional: skip (default — propagate the skip when a hard-input producer is skipped) | run
---

# <number and title, e.g. 1.1 Intent Capture>

<One paragraph on what this stage does and why it exists; with a human approval gate, state the gate name and how re-review works.>

## Steps

1. <which inputs / external material to read>
2. <what to do (intra-stage flow such as the TDD loop and internal judgment goes here)>
3. <write to the paths given in outputs; with an approval gate, wait for approval>
