# whitepaper — step-5 self-review checklist

Run this against the draft before presenting it. Every item must pass; fix what fails.

## Structure

- [ ] **Framing intro** present: states the question, situates the document against any prior
      reports, states scope, and defines notation used throughout.
- [ ] **Setup / method** section establishes everything later sections depend on.
- [ ] For a quantitative subject: evidence states **what theory predicts first**, then the
      **observed** results.
- [ ] Results are in **markdown tables** (quantitative subjects).
- [ ] **Assessment** is a **numbered list**, each item **led by a bold claim sentence**.
- [ ] **Recommendations** section present and actionable.
- [ ] **Bottom line** present and **states the decision** — not a recap of the whole document.

## Evidence & verification

- [ ] **Every quantitative/factual claim is traceable to the source of truth** (code, data,
      literature) — and was checked against it, not restated from memory.
- [ ] Any number produced by a computation the skill ran **cites the script and raw-output
      artifact by name**.
- [ ] A **consistency-checks** note is present where computations were run (what was
      cross-checked, and that it agreed).
- [ ] **Conjecture is explicitly labeled** and never presented as established fact.
- [ ] Where a simulation/computation would have materially strengthened a claim, it was either
      run (with the user's go-ahead) or its absence is noted as an open question.

## Prose bar

- [ ] **Voice is authoritative, quantified, non-hedging** — claims carry magnitudes, not "seems
      to."
- [ ] **Statements are clear and unambiguous.**
- [ ] **Terms are defined before use** and **bolded at first definition.**
- [ ] **Jargon is restrained** — readable by a university-level general-STEM reader who is not a
      subject-matter expert.
- [ ] **LaTeX is used for math** (quantitative subjects) and renders as intended.
- [ ] **Sibling documents are cross-referenced by name and section.**

## House style

- [ ] If the repository has prior reports in a house style, this document **matches their
      conventions** (section naming, notation, cross-reference format).
