# Generic mutability test

Borrow-checker an excellent helper in getting things safely, when the safe Rust is used.
It is well described in RustBook, and, at least, I almost never face the problems I
wouldn't be grateful to be noticed by it.

But in matter of making semantically safe API, when underlying data has no intense
of being so, is a topic, less described in public sources. The last couple of days
I've spend on solving the issue of having parametrization over mutability. So, API
tracks whether `&mut` borrow of one fields makes the same with other, associated
with them.

See the full explanation at
[GitHub pages](https://levitanus.github.io/generic-mutability-test-doc/generic_mutability_test/index.html).

See the source on [GitHub](https://github.com/Levitanus/generic_mutability_test).

## Little Example

```Rust
env_logger::init();
let mut root = Root::new();
let window1 = root.make_child();
// Err: cannot borrow `root` as mutable more than once at a time
// let window2 = root.make_child(); // try to uncomment

let w1_id = window1.get_id(); // Drop window1 and finish &mut borrow of Root
debug!("{}", w1_id);
let id2 = root.make_child().get_id();
let _window1 = root.get_child(w1_id).unwrap();
let _window2 = root.get_child(id2).unwrap(); // OK!

// Err: no method named `make_button` found for struct `Window<'_,
// test::Immutable>` in the current scope. The method was found for
// `Window<'a, test::Mutable>`
// _window1.make_button()

let mut window1 = root.get_child_mut(w1_id).unwrap();
let button = window1.make_button();
let b_id = button.get_id();
let mut frame = window1.make_frame();
let fr_b_id = frame.make_button().get_id();
let f_id = frame.get_id();
// Err: cannot borrow `window1` as mutable more than once at a time
// debug!("button text: {}", button.get_text());

// Err: no method named `get_button` found for struct
// `Window<'_, test::Mutable>` in the current scope
// the method was found for - `Window<'a, test::Immutable>`
// let button = window1.get_button(b_id);
let window1 = root.get_child(w1_id).unwrap();
let frame = window1.get_frame(f_id).unwrap();
let w_b = window1.get_button(b_id).unwrap();
let fr_b = frame.get_button(fr_b_id).unwrap();

debug!("is window button clicked: {}", w_b.is_clicked());
debug!("is frame button clicked: {}", fr_b.is_clicked());
```
