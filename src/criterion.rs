use crate::{BenchOut, LatencyUnit, get_bench_cfg};
use criterion::{
    Criterion, Throughput,
    measurement::{Measurement, ValueFormatter},
};
use std::{
    cell::{Cell, RefCell},
    ops::Deref,
    rc::Rc,
    time::{Duration, Instant},
};

pub struct CriterionOut {
    bench_out: Rc<RefCell<BenchOut>>,
    #[allow(unused)]
    last_val: Cell<u64>,
}

impl CriterionOut {
    pub fn new() -> CriterionOut {
        let cfg = get_bench_cfg();
        let bench_out = BenchOut::new(&cfg);
        CriterionOut {
            bench_out: RefCell::new(bench_out).into(),
            last_val: Cell::new(u64::MAX),
        }
    }

    pub fn bench_out(&self) -> impl Deref<Target = BenchOut> {
        self.bench_out.borrow()
    }

    fn bench_out_rc(&self) -> Rc<RefCell<BenchOut>> {
        self.bench_out.clone()
    }

    // /// Heuristic to detect whether the value being passed to the Measurement is a warmup value.
    // /// Based on the fact that warmup doubles the number of batched executions each iteration until
    // /// the warmup time is exceeded.
    // #[inline(always)]
    // fn is_warmup(&self, val: u64) -> bool {
    //     let last_val = self.last_val.get();
    //     last_val == u64::MAX || val > (last_val as f64 * 1.6) as u64
    // }
}

impl Measurement for CriterionOut {
    type Intermediate = Instant;
    type Value = Duration;

    fn start(&self) -> Self::Intermediate {
        Instant::now()
    }

    fn end(&self, i: Self::Intermediate) -> Self::Value {
        let latency = i.elapsed();
        let mut out = self.bench_out.borrow_mut();
        let val = out.recording_unit.latency_as_u64(latency);
        // if self.is_warmup(val) {
        //     self.last_val.set(val);
        // } else {
        //     out.capture_data(val);
        // }
        out.capture_data(val);
        if out.n() <= 20 {
            println!("{val}");
        }
        latency
    }

    fn add(&self, v1: &Self::Value, v2: &Self::Value) -> Self::Value {
        *v1 + *v2
    }

    fn zero(&self) -> Self::Value {
        Duration::from_secs(0)
    }

    fn to_f64(&self, val: &Self::Value) -> f64 {
        let out = self.bench_out.borrow();
        out.recording_unit.latency_as_f64(*val)
    }

    fn formatter(&self) -> &dyn ValueFormatter {
        &OutFormatter
    }
}

struct OutFormatter;

impl ValueFormatter for OutFormatter {
    fn scale_values(&self, _typical_value: f64, values: &mut [f64]) -> &'static str {
        self.scale_for_machines(values)
    }

    fn scale_throughputs(
        &self,
        _typical: f64,
        _throughput: &Throughput,
        _values: &mut [f64],
    ) -> &'static str {
        "n/a"
    }

    fn scale_for_machines(&self, values: &mut [f64]) -> &'static str {
        let cfg = get_bench_cfg();
        let conversion_factor = cfg.recording_unit().conversion_factor(cfg.reporting_unit());
        for val in values {
            *val *= conversion_factor;
        }

        match cfg.reporting_unit() {
            LatencyUnit::Nano => "nanos",
            LatencyUnit::Micro => "micros",
            LatencyUnit::Milli => "millis",
        }
    }
}

pub fn set_measurement_type() -> Criterion<CriterionOut> {
    let crit_out = CriterionOut::new();
    Criterion::default().with_measurement(crit_out)
}

pub fn enable_measurement(c: &mut Criterion<CriterionOut>) -> Rc<RefCell<BenchOut>> {
    let crit_out = CriterionOut::new();
    let ret = crit_out.bench_out_rc();
    let c0 = Criterion::default().with_measurement(crit_out);
    *c = c0;
    ret
}
