use std::collections::HashMap;

use std::time::{Duration, Instant};

fn timer(f: impl FnOnce()) -> Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}

type HeavyObject = HashMap<usize, Vec<usize>>;

const NUM_ELEMENTS: usize = 1000000;
fn make_heavy_object() -> HeavyObject {
    (1..=NUM_ELEMENTS).map(|v| (v, vec![v])).collect()
}

fn main() {
    println!("Allocating a heavy object");
    let first_heavy_object = make_heavy_object();

    println!("Duplicating that heavy object");
    let second_heavy_object = first_heavy_object.clone();

    // Create a dropper, dropping `HeavyObject`.
    let dropper = dropout::new_dropper();

    // This is a special case for this small test.
    // The closure will explictly drop `first_heavy_object` but also implicitly drop the
    // `dropper` and it will wait for background thread to finish.
    // We don't want to mesure the background drop as it is what we want to avoid.
    // By cloning the dropper, background thread is waited at end of main, not at end of closure.
    let dropper_clone = dropper.clone();
    let dropout_timer = timer(move || dropper_clone.dropout(first_heavy_object));
    let std_timer = timer(move || drop(second_heavy_object));

    println!("Duration of dropout: {:?}", dropout_timer);
    println!("Duration of std drop: {:?}", std_timer);
}
