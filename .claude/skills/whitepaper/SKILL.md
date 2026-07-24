---
name: whitepaper
description: Produce a rigorous written deliverable — an analysis, assessment, or research report — to the user's house standard. Use when the user asks to "write an assessment of X", "produce a research report / whitepaper / write-up on Y", or otherwise requests a written analytical DOCUMENT. Do NOT use for casual "analyze this line of code" asks that want an inline answer, not a document.
---

# whitepaper

Produce a rigorous written deliverable — an **analysis**, **assessment**, or **research
report** — to a fixed quality bar. These three are **one deliverable pattern**, not three: the
same structure and the same bar apply to all. Emphasis shifts only in degree — a *research
report* leans harder on gathering and citing external sources; an *assessment* leans harder on
evaluation and recommendations — but the skeleton and the standards below do not fork.

This skill is domain-general. Nothing here assumes a particular subject matter. Where the
subject is quantitative, quantitative conventions (LaTeX, results tables) apply; where it is
not, they do not.

## Workflow

Follow these steps in order. Steps 1, 2, and 6 are **gates**: do not pass one until the user
has explicitly signed off.

### 1. Scope — always grill first

Before any planning or drafting, run a full `/grilling`-style interrogation to nail the scope.
Interview relentlessly, **one question at a time**, walking each branch of the decision tree and
resolving dependencies one by one; provide a recommended answer with each question. Look up
*facts* in the environment (filesystem, code, tools) rather than asking; put every *decision* to
the user. Pin down at least: the precise question the document answers, the intended reader,
what is in and out of scope, and what counts as admissible evidence.

**Do not proceed to planning until the user confirms shared understanding.**

### 2. Plan — written, saved, approved

Write a plan to disk and get explicit approval before drafting. **Ask the user where the plan,
the working draft, and the final document should each live** — do not assume a directory
convention. The plan must contain:

- the **section skeleton** for this document (adapt the default spine below to the subject);
- the **evidence/verification plan**: for each major claim, the source of truth it will be
  checked against (code, data, literature); and which claims, if any, warrant a new
  simulation or computation;
- **sibling documents to cross-reference**, if any.

**Do not draft until the user approves the plan.**

### 3. Evidence & verification

- **Verify every quantitative or factual claim against the existing source of truth** — the
  code, the data, the primary literature. Do not restate numbers or definitions from memory;
  recompute or re-read them from the source.
- **New simulations/computations are not mandatory, but propose them proactively when they
  would materially strengthen a claim.** Run them only on the user's go-ahead. When a claim
  rests on a computation you ran, the document must **cite the script and its raw-output
  artifact by name**, and include a short **consistency-checks** note (what you cross-checked
  and that it agreed).
- **Never blur verified fact and conjecture.** Anything not established by evidence is labeled
  as conjecture, hypothesis, or open question — explicitly, in the text.

### 4. Draft

Write the document to the agreed draft path, following the **deliverable spec** below.

### 5. Self-review

Before presenting the draft, load `reference/self-review.md` and check the document against
every item. Fix what fails. See `reference/exemplar.md` for an annotated model of the target
style.

### 6. Iterate to sign-off, then finalize

Present the draft, incorporate the user's feedback, and iterate. **Write the final document to
the agreed final path only after the user's explicit approval.**

## Deliverable spec

### Default spine (adapt to the subject)

1. **Framing intro** — situate the document (its question, and its relationship to any prior
   reports), state scope, and define notation used throughout.
2. **Setup / method** — the model, data, procedure, or definitions the rest depends on.
3. **Evidence** — where the subject is quantitative, state *what theory predicts* first, then
   the *observed* results, and present results in **markdown tables**. Where it is not, present
   the gathered evidence directly.
4. **Assessment** — the findings, as a **numbered list, each item led by a bold claim
   sentence**, followed by its supporting evidence.
5. **Recommendations** — what to do, given the findings.
6. **Bottom line** — a short closing that **states the decision**, not a summary of everything
   above.

If the repository already contains prior reports in a house style, **match their conventions**
(section naming, notation, cross-reference format) in preference to the generic defaults here.

### Prose bar

1. **Authoritative, quantified, non-hedging voice.** Findings assert and attach magnitudes
   ("5–20× the clean error"), not "seems to" or "appears that."
2. **Numbered findings, each led by a bold claim sentence**, then its evidence.
3. **Clear and unambiguous statements.**
4. **Define terms before using them.**
5. **Bold terms at first definition.**
6. **Avoid excessive jargon.** Prefer terminology understandable by a reader with a
   university-level general STEM education who is not necessarily an expert in the topic's
   subject matter.
7. **LaTeX math wherever the subject is quantitative; markdown tables for results.**
8. **Cross-reference sibling documents by name and section** (e.g. "§5.1 of
   `Assessment_Improved_Estimators.md`").
9. **Verified fact and conjecture never blur** — conjecture is explicitly labeled.
10. **A "Bottom line" that states the decision**, not a summary of everything.
