use crate::multi::BenchOut;

impl<const K: usize> BenchOut<K> {
    pub fn collect_data(&mut self, mut src: [impl Iterator<Item = u64>; K]) {
        for (k, b) in &mut self.arr.iter_mut().enumerate() {
            b.collect_data(&mut src[k]);
        }
    }

    pub fn print(&self) {
        println!("{self:?}");
    }
}
