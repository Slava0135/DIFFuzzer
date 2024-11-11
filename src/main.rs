use rand::{rngs::StdRng, SeedableRng};

mod abstract_fs;
mod mutator;

fn main() {
    let mut rng = StdRng::seed_from_u64(123);
    let seq = mutator::generate_new(&mut rng, 10);
    println!("{seq:?}");
}
