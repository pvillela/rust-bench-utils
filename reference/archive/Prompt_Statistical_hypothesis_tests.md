I am concerned that several of the hypothesis tests and confidence intervals defined in `BenchOut` and `Comp` may not be valid or useful for batched benchmarks.

I need an assessment of the hypothesis testing and conficence interval methods, recommendations regarding alternatives better suited for batched benchmarks, and an implementation plan. In particular, consider the appropriateness of    alternatives based on bootstrapping.

References @reference/Reconciliation_notes_Contamination_vs_Fable_mean_estimators.md and @reference/Histogram_vs_Vec_sample_storage.md should be relevant to your analysis as they can inform considerations on whether to modify the sample data storage approach in BenchOut.