# dropout

A small library to send your value to be dropped out of your thread.
Inspired by [this article](https://abramov.io/rust-dropping-things-in-another-thread) by Aaron Abramov and [defer-drop crate](https://github.com/Lucretiel/defer-drop)

## Simple example

```rust
// See examples/demo.rs


type HeavyObject = HashMap<usize, Vec<usize>>;

fn make_heavy_object() -> HeavyObject {
    (1..=NUM_ELEMENTS).map(|v| (v, vec![v])).collect()
}

println!("Allocating a heavy object");
let first_heavy_object = make_heavy_object();

println!("Duplicating that heavy object");
let second_heavy_object = first_heavy_object.clone();

// Create a dropper, dropping `Vec<Vec<String>>`.
let dropper = dropout::new_dropper();

// This is a special case for this small test.
// The closure will explictly drop vec1 but also implicitly drop the `dropper`
// and we don't want that as, at drop, dropper wait for background thread.
// We don't want to mesure that.
let dropper_clone = dropper.clone();
let dropout_timer = timer(move || dropper_clone.dropout(first_heavy_object));
let std_timer = timer(move || drop(second_heavy_object));

println!("Duration of dropout: {:?}", dropout_timer);
println!("Duration of std drop: {:?}", std_timer);
```

Output is (can be):

```
Allocating a heavy object
Duplicating that heavy object
Duration of dropout: 7.479µs
Duration of std drop: 50.864814ms
```

## Difference with defer-drop

### API

Defer-drop use one global background thread to drop any type.
- All values are send to the background thread through a `Box`.
- You have to create a `DeferDrop` which will wrap your value and send it to drop thread when wrapper is drop.

Dropout take the opposite direction:
- You create a Dropper which accept (and take ownership) of a `T`.
- You have one thread per dropper.
- You handle `T` object and explicitly defer the drop at end.

So your manipulating `T` value instead of `DeferDrop<T>`.
But you have to be explicit about droping the value.

### Guarantees

Defer-drop doesn't guaranty values are actually dropped as the main thread may finish before drop thread has dropped all the values.
`Dropper` wait (and so, block at drop) for the background thread to finish (and drop all values) before being drop.

### Licensing differences

- defer-drop is licensed under MPL-2.0
- dropout is licensed under MIT


```rust
// In external library

pub trait MyTrait {
  fn get_u32(&self) -> u32;
}

fn do_stuff_with_object<T: MyTrait>(object: T) {
  ...
  drop(object)
}

// In user library
use external_library::{MyTrait, do_stuff_with_object};

struct MyObject {}
impl MyTrait for MyObject {
  fn get_u32(self) -> u32 {
    5
  }
};

struct DeferedMyObject(DeferedDrop<MyObject>)

impl MyTrait for DeferedMyObject {
  fn get_u32(&self) -> u32 {
    self.0.get_u32()
  }
}

fn main() {
  do_stuff_with_object(MyObject{}); // Works
  do_stuff_with_object(DeferDrop::new(MyObject{})); // Doesn't work as DeferDrop doesn't impl MyTrait
  do_stuff_with_object(DeferedMyObject(DeferDrop::new(MyObject{}))); // Works
}
```

And this is almost impossible if trait consume self as `Box<Self>`

```rust
pub trait MyTrait {
  fn get_as_u32(self: Box<Self>) -> u32;
}

fn do_stuff_with_u32(object: Box<dyn MyTrait>) {
  let value = object.get_as_u32();
  ...
}
```

## Notes

Dropout (as defer-drop) is not a silver bullet. Sending the value to another thread is costly and it may be counter productive.
Always do som performance profiling before using such crates.

Dropped values are enqueued in an unbounded channel to be consumed by dropping thread; if you produce more values
than the thread can handle, this will cause unbounded memory consumption.
There is currently no way for the thread to signal or block if it is overwhelmed.

All of the standard non-determinism threading caveats apply here.
The objects are guaranteed to be destructed in the order received through a channel, which means that objects sent from a single thread will be destructed in order.
However, there is no guarantee about the ordering of interleaved values from different threads.

Additionally, there are no guarantees about how long the values will be queued before being dropped.
However, when the dropper is droped, it waits for the background thread to finish, so this is guarented that all objects will be droped at some point.

