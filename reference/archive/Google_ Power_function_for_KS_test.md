# Google: Power function for KS test.

## Question 1

**write a rust function to calculate the power of a ks test for a given cdf.**

rust

```
use rand::distributions::{Distribution, Uniform};
use rand::thread_rng;

/// Calculates the statistical power of a one-sample Kolmogorov-Smirnov (KS) test.
/// 
/// # Arguments
/// * `true_sampler` - A closure that generates a random sample `f64` from the true (alternative) distribution.
/// * `null_cdf` - A closure representing the Cumulative Distribution Function under the null hypothesis.
/// * `sample_size` - The number of data points per simulated dataset ($n$).
/// * `alpha` - The significance level (Type I error rate, e.g., 0.05).
/// * `simulations` - The number of Monte Carlo replications to run (e.g., 10000).
/// 
/// # Returns
/// * `f64` - The estimated statistical power (between 0.0 and 1.0).
pub fn calculate_ks_power<F, G>(
    mut true_sampler: F,
    null_cdf: G,
    sample_size: usize,
    alpha: f64,
    simulations: usize,
) -> f64
where
    F: FnMut() -> f64,
    G: Fn(f64) -> f64,
    {
    // 1. Calculate critical value for the KS test statistic
    let critical_value = calculate_ks_critical_value(sample_size, alpha);
    let mut rejections = 0;

    // 2. Run Monte Carlo simulations
    for _ in 0..simulations {
        // Generate dataset under the alternative hypothesis
        let mut sample: Vec<f64> = (0..sample_size).map(|_| true_sampler()).collect();
        
        // Sort sample to compute the Empirical CDF (ECDF)
        sample.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Calculate the maximum distance D between ECDF and null CDF
        let mut max_d: f64 = 0.0;
        let n_f64 = sample_size as f64;

        for (i, &x) in sample.iter().enumerate() {
            let ecdf_minus = i as f64 / n_f64;
            let ecdf_plus = (i + 1) as f64 / n_f64;
            let cdf_val = null_cdf(x);

            let d1 = (ecdf_plus - cdf_val).abs();
            let d2 = (cdf_val - ecdf_minus).abs();

            if d1 > max_d { max_d = d1; }
            if d2 > max_d { max_d = d2; }
        }

        // 3. Reject null if the statistic exceeds the critical value
        if max_d > critical_value {
            rejections += 1;
        }
    }

    // 4. Return the empirical power
    rejections as f64 / simulations as f64
}

/// Approximates the critical value for the KS test statistic ($D_{n,\alpha}$).
/// Uses standard asymptotic approximation for larger samples: $c(\alpha) / \sqrt{n}$
fn calculate_ks_critical_value(n: usize, alpha: f64) -> f64 {
    let c_alpha = (-0.5 * (alpha / 2.0).ln()).sqrt();
    c_alpha / (n as f64).sqrt()
}

fn main() {
    let mut rng = thread_rng();
    let die = Uniform::new(0.0, 1.0);
    
    // Alternative distribution: Uniform(0.1, 1.1) [Shifted distribution]
    let true_sampler = || die.sample(&mut rng) + 0.1;
    
    // Null distribution: Standard Uniform(0.0, 1.0)
    let null_cdf = |x: f64| {
        if x < 0.0 { 0.0 }
        else if x > 1.0 { 1.0 }
        else { x }
    };

    let power = calculate_ks_power(true_sampler, null_cdf, 50, 0.05, 10_000);
    println!("Estimated KS Test Power: {:.4}", power);
}
```

Use code with caution.

Dependencies

Add the `rand` crate to your `Cargo.toml` file to run the simulation logic:

toml

```
[dependencies]
rand = "0.8"
```

Use code with caution.

Implementation Details

- **Empirical CDF Mapping**: The function sorts each generated sample to build the step-wise empirical distribution.
- **Two-Sided Deviation**: It checks the absolute difference at both the bottom (\(i/n\)) and the top (\((i+1)/n\)) of each step against the target analytical CDF to accurately capture the supremum distance \(D\).
- **Asymptotic Approximation**: The critical value helper leverages the Kolmogorov distribution limit (\(c(\alpha) / \sqrt{n}\)), which functions optimally for sample sizes \(n > 35\).

Further Exploration

If you want to refine this script, tell me:

- Do you need **exact critical values** for very small sample sizes (\(n < 35\))?
- Should the simulation support **multi-threaded execution** via the Rayon crate to speed up heavy loops?
- Do you want to pass **parametrised distributions** from the `statrs` crate instead of custom closures? [[1](https://arxiv.org/html/2410.19146v1)]



## Question 2

**\* The simulation should support multi-threaded execution via the Rayon crate to speed up heavy loops. **

*** Pass parametrised distributions from the statrs crate instead of custom closures.**



Here is the optimized, multi-threaded Rust implementation using the `rayon` and `statrs` crates.

Multi-Threaded Rust Implementation

rust

```
use rayon::prelude::*;
use statrs::distribution::{Continuous, ContinuousCDF};

/// Calculates the statistical power of a one-sample Kolmogorov-Smirnov (KS) test.
/// Utilizes parallel iteration via Rayon for fast Monte Carlo simulations.
/// 
/// # Arguments
/// * `true_dist` - The actual underlying distribution from `statrs` (implements Continuous).
/// * `null_dist` - The assumed null hypothesis distribution from `statrs` (implements ContinuousCDF).
/// * `sample_size` - The number of data points per simulated dataset ($n$).
/// * `alpha` - The significance level (Type I error rate, e.g., 0.05).
/// * `simulations` - The number of Monte Carlo replications to run.
/// 
/// # Returns
/// * `f64` - The estimated statistical power (between 0.0 and 1.0).
pub fn calculate_ks_power_parallel<T, N>(
    true_dist: &T,
    null_dist: &N,
    sample_size: usize,
    alpha: f64,
    simulations: usize,
) -> f64
where
    T: Continuous<f64, f64> + Sync,
    N: ContinuousCDF<f64, f64> + Sync,
{
    // 1. Compute critical value for the test
    let critical_value = calculate_ks_critical_value(sample_size, alpha);

    // 2. Parallel execution over simulations using Rayon
    let rejections: usize = (0..simulations)
        .into_par_iter()
        .map(|_| {
            let mut rng = rand::thread_rng();
            
            // Sample directly from the statrs continuous distribution
            let mut sample: Vec<f64> = true_dist.sample_iter(&mut rng)
                .take(sample_size)
                .collect();

            // Sort sample to compute the Empirical CDF (ECDF)
            sample.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let mut max_d: f64 = 0.0;
            let n_f64 = sample_size as f64;

            // Calculate the supreme distance D between ECDF and null CDF
            for (i, &x) in sample.iter().enumerate() {
                let ecdf_minus = i as f64 / n_f64;
                let ecdf_plus = (i + 1) as f64 / n_f64;
                let cdf_val = null_dist.cdf(x);

                let d1 = (ecdf_plus - cdf_val).abs();
                let d2 = (cdf_val - ecdf_minus).abs();

                if d1 > max_d { max_d = d1; }
                if d2 > max_d { max_d = d2; }
            }

            // Return 1 if null is rejected, else 0
            if max_d > critical_value { 1 } else { 0 }
        })
        .sum();

    // 3. Return final calculated power
    rejections as f64 / simulations as f64
}

/// Approximates the critical value for the KS test statistic ($D_{n,\alpha}$).
/// Uses standard asymptotic approximation for larger samples: $c(\alpha) / \sqrt{n}$
fn calculate_ks_critical_value(n: usize, alpha: f64) -> f64 {
    let c_alpha = (-0.5 * (alpha / 2.0).ln()).sqrt();
    c_alpha / (n as f64).sqrt()
}

fn main() {
    use statrs::distribution::{Normal, Uniform};

    // Define alternative distribution: Normal(mean=0.3, std_dev=1.0)
    let true_dist = Normal::new(0.3, 1.0).unwrap();

    // Define null distribution: Standard Normal(mean=0.0, std_dev=1.0)
    let null_dist = Normal::new(0.0, 1.0).unwrap();

    let sample_size = 40;
    let alpha = 0.05;
    let simulations = 50_000; // Large simulation count to leverage multi-threading

    let power = calculate_ks_power_parallel(&true_dist, &null_dist, sample_size, alpha, simulations);
    println!("Parallel KS Test Power: {:.4}", power);
    
    // Example 2: Uniform vs Uniform
    let true_uniform = Uniform::new(0.1, 1.1).unwrap();
    let null_uniform = Uniform::new(0.0, 1.0).unwrap();
    let power_uni = calculate_ks_power_parallel(&true_uniform, &null_uniform, sample_size, alpha, simulations);
    println!("Uniform Shift KS Test Power: {:.4}", power_uni);
}
```

Use code with caution.

Required Cargo Dependencies

Update your `Cargo.toml` to scale out the math calculations and utilize multi-core scaling:

toml

```
[dependencies]
rand = "0.8"
rayon = "1.10"
statrs = "0.17"
```

Use code with caution.

Architectural Optimizations

- **Trait Constraints**: The function accepts any distribution types conforming to `statrs::distribution::Continuous` (for generation via `.sample_iter()`) and `statrs::distribution::ContinuousCDF` (for evaluating `.cdf()`).
- **Thread Safety**: Trait bounds require `Sync` so references to your distributions can be safely shared across Rayon worker threads without cloning the distribution parameters. [[1](https://developers.redhat.com/blog/2021/04/30/how-rust-makes-rayons-data-parallelism-magical)]
- **Thread-Local RNGs**: Initializing `rand::thread_rng()` inside the `.map()` step ensures that threads do not contention-lock a single shared random state, preventing lock bottlenecks. [[1](https://cjwebb.com/parallel-monte-carlo-rust/)]
- **Memory Localized Operations**: Vector generation, sorting, and comparison logic execute entirely within isolated stackframes inside Rayon's threadpool to achieve maximum cache performance.

Would you like to extend this engine to compute **two-sample KS test** powers, or perhaps implement an **automated grid-search loop** to determine the minimum required sample size for a target power level (e.g., \(1 - \beta = 0.80\))?



## Question 3

**Implement an automated grid-search loop to determine the minimum required sample size for a target power level .**

Here is the extended implementation. It wraps the previous parallel power calculator in a search loop (using a fast binary search instead of a slow linear grid search) to find the exact minimum sample size required to hit your target power.

Rust Implementation (Power Grid Search)

rust

```
use rayon::prelude::*;
use statrs::distribution::{Continuous, ContinuousCDF};

/// Finds the minimum sample size ($n$) required to achieve a target power level.
/// Uses a binary search approach over sample sizes to minimize execution time.
/// 
/// # Arguments
/// * `true_dist` - The actual underlying distribution from `statrs`.
/// * `null_dist` - The assumed null hypothesis distribution from `statrs`.
/// * `alpha` - The significance level (Type I error rate, e.g., 0.05).
/// * `target_power` - The desired power level (e.g., 0.80).
/// * `simulations` - The number of Monte Carlo loops per test (higher = more precise size estimation).
/// * `min_n` - Lower bound for the sample size search range (must be > 0, e.g., 10).
/// * `max_n` - Upper bound for the sample size search range (e.g., 1000).
/// 
/// # Returns
/// * `Result<usize, String>` - The minimum sample size, or an error if target power is unreachable.
pub fn find_minimum_sample_size<T, N>(
    true_dist: &T,
    null_dist: &N,
    alpha: f64,
    target_power: f64,
    simulations: usize,
    min_n: usize,
    max_n: usize,
) -> Result<usize, String>
where
    T: Continuous<f64, f64> + Sync,
    N: ContinuousCDF<f64, f64> + Sync,
{
    let mut low = min_n;
    let mut high = max_n;
    let mut optimal_n = None;

    // Fast-check upper bound first to ensure target power is even achievable in this window
    let max_power = calculate_ks_power_parallel(true_dist, null_dist, high, alpha, simulations);
    if max_power < target_power {
        return Err(format!(
            "Target power of {} cannot be reached within max_n={}. Max power achieved was {:.4}", 
            target_power, high, max_power
        ));
    }

    // Binary search logic across the sample size space
    while low <= high {
        let mid = low + (high - low) / 2;
        let current_power = calculate_ks_power_parallel(true_dist, null_dist, mid, alpha, simulations);

        println!("Testing Sample Size (n): {:<4} | Achieved Power: {:.4}", mid, current_power);

        if current_power >= target_power {
            optimal_n = Some(mid);
            // Try to find a smaller sample size that still satisfies the constraint
            if mid == 0 { break; }
            high = mid - 1;
        } else {
            // Power is too low; we need a larger sample size
            low = mid + 1;
        }
    }

    optimal_n.ok_or_else(|| "Failed to converge on a valid sample size.".to_string())
}

/// The multi-threaded power calculator from the previous step
pub fn calculate_ks_power_parallel<T, N>(
    true_dist: &T,
    null_dist: &N,
    sample_size: usize,
    alpha: f64,
    simulations: usize,
) -> f64
where
    T: Continuous<f64, f64> + Sync,
    N: ContinuousCDF<f64, f64> + Sync,
{
    let critical_value = calculate_ks_critical_value(sample_size, alpha);

    let rejections: usize = (0..simulations)
        .into_par_iter()
        .map(|_| {
            let mut rng = rand::thread_rng();
            let mut sample: Vec<f64> = true_dist.sample_iter(&mut rng).take(sample_size).collect();
            sample.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let mut max_d: f64 = 0.0;
            let n_f64 = sample_size as f64;

            for (i, &x) in sample.iter().enumerate() {
                let ecdf_minus = i as f64 / n_f64;
                let ecdf_plus = (i + 1) as f64 / n_f64;
                let cdf_val = null_dist.cdf(x);

                let d1 = (ecdf_plus - cdf_val).abs();
                let d2 = (cdf_val - ecdf_minus).abs();

                if d1 > max_d { max_d = d1; }
                if d2 > max_d { max_d = d2; }
            }

            if max_d > critical_value { 1 } else { 0 }
        })
        .sum();

    rejections as f64 / simulations as f64
}

fn calculate_ks_critical_value(n: usize, alpha: f64) -> f64 {
    let c_alpha = (-0.5 * (alpha / 2.0).ln()).sqrt();
    c_alpha / (n as f64).sqrt()
}

fn main() {
    use statrs::distribution::Normal;

    // Scenario: Null is Standard Normal, True is a subtle mean shift (0.25)
    let null_dist = Normal::new(0.0, 1.0).unwrap();
    let true_dist = Normal::new(0.25, 1.0).unwrap();

    let alpha = 0.05;
    let target_power = 0.80;      // Standard target power (80%)
    let simulations = 20_000;    // Precision vs Speed tradeoff

    println!("Searching for minimum sample size to achieve 80% power...");
    match find_minimum_sample_size(&true_dist, &null_dist, alpha, target_power, simulations, 10, 500) {
        Ok(min_n) => println!("\nSuccess! Minimum required sample size (n): {}", min_n),
        Err(e) => println!("\nSearch failed: {}", e),
    }
}
```

Use code with caution.

Efficiency Enhancements

- **Binary vs Grid Search**: Instead of evaluating every sample size sequentially (a linear grid search), binary search chops the search space in half at each iteration. Finding a sample size between 1 and 500 takes roughly 9 steps instead of up to 500 steps.
- **Early Fail Termination**: The algorithm tests the maximum bounds constraint immediately before initiating the loop, saving computational execution if the configuration is impossible.
- **Preserved Parallelism**: Rayon handles the inner loop (the heavy Monte Carlo simulation), while the sequential loop coordinates the search direction.

If you want to advance this pipeline further, let me know if you would like to:

- Generate a **CSV export payload** to chart a full Power curve (n versus power)?
- Factor in a **Type II error (β) threshold mapping** explicitly into your logs?
- Implement a **two-sample version** where group sizes might scale unevenly?

