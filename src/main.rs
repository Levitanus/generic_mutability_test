//! Example of restricting statically-checked mutability for types,
//! that are not originally designed for this (e.g. FFI wrappers etc)
//! or which uses interior mutability, but still need verbose
//! API interface.
//!
//! # The problem
//!
//! Let's start from the [monkey_ffi] module.
//!
//! This is an example of extern C API, that looks like object-oriented,
//! but is not safe and not guarantees the existence of objects, as well
//! as their relations.
//!
//! It looks like tree-structure:
//!
//! ```ignore
//! Root
//! ----Window
//! ----Frame
//! ----|----FrameButton
//! ----WindowButton
//! ```
//!
//! Ideally, only one object at time should be mutable,
//! and we have to guarantee, that parent lives at least as long as child.
//!
//! But the problem is, that with Rc, or just pointers, on every
//! fold level parent mutability state is lost. We can not make easily
//! two versions of [Window]: one, that keeps `&'a mut Root` and other,
//! that keeps `&'a Root`, just because we will need to type whole the
//! implementation block twice.
//!
//! But the fact, that `Self`, `&Self` and `&mut Self` are complete different types and this
//! [discussion](https://users.rust-lang.org/t/generic-mutability-parameters/16837/23)
//! pointed me the solution.
//!
//! Actually, mutability in `rust` is tri-state: immutable, mutable, and that, which
//! we don't care about. Which we want to be «generic». So, let's start from declaring
//! types, that will represent all the three states: two concrete structs and one trait.
//!
//! ```
//! trait ProbablyMutable {
//! fn is_mutable(&self) -> bool;
//! }
//! struct Mutable;
//! impl ProbablyMutable for Mutable {
//!     fn is_mutable(&self) -> bool {
//!         true
//!     }
//! }
//! struct Immutable;
//! impl ProbablyMutable for Immutable {
//!     fn is_mutable(&self) -> bool {
//!         false
//!     }
//! }
//! ```
//!
//! Then we will use these types as markers for future parametrization.
//!
//! Now let's make a skeleton of object structure and ensure that no child will
//! outlive their parents.
//!
//! ```
//! # trait ProbablyMutable;
//! struct Root;
//! ```
//!
//! Here we use [PhantomData] to keep «generic» part
//! outside of any concrete parent object.
//! Later, it will help not to have in a deep children structure
//! scary constructions, like:
//! `SecondChild<&mut Parent, &mut FirstChild<&mut Parent>>`
//!
//! ```
//! # trait ProbablyMutable;
//! # struct Root;
//! struct Window<'a, T: ProbablyMutable> {
//!     id: usize,
//!     name: String,
//!     frames_amount: usize,
//!     buttons_amount: usize,
//!     root: &'a Root,
//!     mutability: PhantomData<T>,
//! }
//!
//! struct Frame<'a, T: ProbablyMutable> {
//!     window: &'a Window<'a, T>,
//!     id: usize,
//!     width_px: Option<u16>,
//!     buttons_amount: usize,
//! }
//!
//! struct WindowButton<'a, T: ProbablyMutable> {
//!     id: usize,
//!     text: String,
//!     parent: &'a Window<'a, T>,
//! }
//!
//! struct FrameButton<'a, T: ProbablyMutable> {
//!     id: usize,
//!     text: String,
//!     parent: &'a Frame<'a, T>,
//! }
//! ```
//!
//! since there are two different functions sets for buttons
//! I decided to consider them as different classes with
//! the single interface (trait) [Button].
//!
//! Theoretically, it should be possible to make a single struct,
//! that keeps differs of types in enum. But for the moment, it
//! seemed to me like an overhead.
//!
//! Considering the generic mutation, I implement it like implementation
//! of three different types:
//! - `struct<T: ProbablyMutable>` for functions, that should be generic
//! in their mutability.
//! - `struct<Mutable>` for functions, that require object to be mutable.
//! - `struct<Immutable>` for functions, that require object to be immutable.
//!
//! ```
//! # trait ProbablyMutable;
//! # struct Root;
//! # struct Frame<T: ProbablyMutable>;
//! # struct Window<T: ProbablyMutable>;
//! impl<'a, T: ProbablyMutable> Window<'a, T> {
//!     fn new(root: &'a Root, id: usize) -> Option<Self> {todo!()}
//!     fn get_id(&self) -> usize {todo!()}
//!     fn get_name(&self) -> &String {todo!()}
//!     fn get_width(&self) -> u16 {todo!()}
//! }
//! impl<'a> Window<'a, Immutable> {
//!     fn get_frame(&self, id: usize) -> Option<Frame<Immutable>> {todo!()}
//!     fn get_button(&self, id: usize) -> Option<WindowButton<Immutable>> {todo!()}
//! }
//! impl<'a> Window<'a, Mutable> {
//!     fn set_name(&mut self, name: impl Into<String>) {todo!()}
//!     fn make_frame(&mut self) -> Frame<Mutable> {todo!()}
//!     fn make_button(&mut self) -> WindowButton<Mutable> {todo!()}
//! }
//! ```
use std::marker::PhantomData;

use log::debug;

mod monkey_ffi {
    //! module that imitates some FFI functions set.
    //!
    //! ```ignore
    //! fn make_window() -> usize;
    //! fn get_window(window_id: usize) -> usize;
    //! fn make_frame(_window_id: usize) -> usize;
    //! fn make_window_button(_window_id: usize) -> usize;
    //! fn make_frame_button(_window_id: usize) -> usize;
    //! fn window_button_is_clicked(_window_id: usize, _button_id: usize) -> bool;
    //! fn window_button_click(window_id: usize, button_id: usize);
    //! fn window_button_set_text(window_id: usize, button_id: usize, text: &String);
    //! fn frame_button_is_clicked(_frame_id: usize, _button_id: usize) -> bool;
    //! fn frame_button_click(frame_id: usize, button_id: usize);
    //! fn frame_button_set_text(frame_id: usize, button_id: usize, text: &String);
    //! ```

    use log::info;

    static mut MONKEY_WINDOWS_AMOUNT: usize = 0;
    static mut MONKEY_FRAMES_AMOUNT: usize = 0;
    static mut MONKEY_WINDOW_BUTTONS_AMOUNT: usize = 0;
    static mut MONKEY_FRAME_BUTTONS_AMOUNT: usize = 0;

    pub fn make_window() -> usize {
        unsafe {
            MONKEY_WINDOWS_AMOUNT += 1;
            let id = MONKEY_WINDOWS_AMOUNT;
            info!("Created Window with id: {}", id);
            id
        }
    }
    pub fn get_window(window_id: usize) -> usize {
        unsafe {
            let id = MONKEY_WINDOWS_AMOUNT;
            if window_id > id {
                return 0;
            }
            info!("Returned Window with id: {}", id);
            id
        }
    }
    pub fn make_frame(_window_id: usize) -> usize {
        unsafe {
            MONKEY_FRAMES_AMOUNT += 1;
            let id = MONKEY_FRAMES_AMOUNT;
            info!("Created Frame with id: {}", id);
            id
        }
    }
    pub fn make_window_button(_window_id: usize) -> usize {
        unsafe {
            MONKEY_WINDOW_BUTTONS_AMOUNT += 1;
            let id = MONKEY_WINDOW_BUTTONS_AMOUNT;
            info!("Created WindowButton with id: {}", id);
            id
        }
    }
    pub fn make_frame_button(_window_id: usize) -> usize {
        unsafe {
            MONKEY_FRAME_BUTTONS_AMOUNT += 1;
            let id = MONKEY_FRAME_BUTTONS_AMOUNT;
            info!("Created FrameButton with id: {}", id);
            id
        }
    }
    pub fn window_button_is_clicked(_window_id: usize, _button_id: usize) -> bool {
        false
    }

    pub fn window_button_click(window_id: usize, button_id: usize) {
        info!(
            "WindowButton clicked! window={}, button={}",
            window_id, button_id
        );
    }
    pub fn window_button_set_text(window_id: usize, button_id: usize, text: &String) {
        info!(
            "WindowButton text set to: {}! window={}, button={}",
            text, window_id, button_id
        );
    }
    pub fn frame_button_is_clicked(_frame_id: usize, _button_id: usize) -> bool {
        false
    }

    pub fn frame_button_click(frame_id: usize, button_id: usize) {
        info!(
            "FrameButton clicked! window={}, button={}",
            frame_id, button_id
        );
    }
    pub fn frame_button_set_text(frame_id: usize, button_id: usize, text: &String) {
        info!(
            "FrameButton text set to: {}! window={}, button={}",
            text, frame_id, button_id
        );
    }
}

trait ProbablyMutable {
    fn is_mutable(&self) -> bool;
}
struct Mutable;
impl ProbablyMutable for Mutable {
    fn is_mutable(&self) -> bool {
        true
    }
}
struct Immutable;
impl ProbablyMutable for Immutable {
    fn is_mutable(&self) -> bool {
        false
    }
}

struct Root;
impl Root {
    fn new() -> Self {
        Self {}
    }
    fn make_child(&mut self) -> Window<Mutable> {
        let id = monkey_ffi::make_window();
        let child = Window::new(self, id).expect("Should be initialized.");
        child
    }
    fn get_child(&self, id: usize) -> Option<Window<Immutable>> {
        Window::new(self, id)
    }
    fn get_child_mut(&mut self, id: usize) -> Option<Window<Mutable>> {
        Window::new(self, id)
    }
}

struct Window<'a, T: ProbablyMutable> {
    id: usize,
    name: String,
    frames_amount: usize,
    buttons_amount: usize,
    root: &'a Root,
    mutability: PhantomData<T>,
}
impl<'a, T: ProbablyMutable> Window<'a, T> {
    fn new(root: &'a Root, id: usize) -> Option<Self> {
        match monkey_ffi::get_window(id) {
            0 => None,
            x => Self {
                id,
                name: String::default(),
                frames_amount: 0,
                buttons_amount: 0,
                root,
                mutability: PhantomData,
            }
            .into(),
        }
    }
    fn get_id(&self) -> usize {
        self.id
    }
    fn get_name(&self) -> &String {
        &self.name
    }
    fn get_width(&self) -> u16 {
        debug!("Some FFI call to het width");
        400
    }
}
impl<'a> Window<'a, Immutable> {
    fn get_frame(&self, id: usize) -> Option<Frame<Immutable>> {
        Frame::new(self, id)
    }
    fn get_button(&self, id: usize) -> Option<WindowButton<Immutable>> {
        Button::new(self, id)
    }
}
impl<'a> Window<'a, Mutable> {
    fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }
    fn make_frame(&mut self) -> Frame<Mutable> {
        debug!("Some FFI magic");
        let id = monkey_ffi::make_frame(self.get_id());
        self.frames_amount += 1;
        let sub_child = Frame::new(self, id).expect("Should be created and valid.");
        sub_child
    }
    fn make_button(&mut self) -> WindowButton<Mutable> {
        debug!("Some FFI magic");
        let id = self.buttons_amount;
        self.buttons_amount += 1;
        let button = WindowButton::new(&*self, id).expect("Should be created and valid.");
        button
    }
}

struct Frame<'a, T: ProbablyMutable> {
    window: &'a Window<'a, T>,
    id: usize,
    width_px: Option<u16>,
    buttons_amount: usize,
}
impl<'a, T: ProbablyMutable> Frame<'a, T> {
    fn new(window: &'a Window<T>, id: usize) -> Option<Self> {
        let id = monkey_ffi::make_frame(id);
        match id {
            0 => None,
            id => Self {
                window,
                id,
                width_px: None,
                buttons_amount: 0,
            }
            .into(),
        }
    }
    fn get_id(&self) -> usize {
        self.id
    }
    fn get_width(&self) -> u16 {
        match self.width_px {
            None => self.window.get_width(),
            Some(width) => width,
        }
    }
}
impl<'a> Frame<'a, Immutable> {
    fn get_button(&self, id: usize) -> Option<FrameButton<Immutable>> {
        FrameButton::new(self, id)
    }
}
impl<'a> Frame<'a, Mutable> {
    fn set_width(&mut self, width_px: impl Into<u16>) {
        self.width_px = Some(width_px.into())
    }
    fn make_button(&mut self) -> FrameButton<Mutable> {
        debug!("Some FFI magic");
        let id = self.buttons_amount;
        self.buttons_amount += 1;
        let button = FrameButton::new(&*self, id).expect("Should be created and valid.");
        button
    }
}

trait Button<T: ProbablyMutable>
where
    Self: Sized,
{
    type Parent;
    fn new(parent: Self::Parent, id: usize) -> Option<Self>;
    fn get_id(&self) -> usize;
    fn is_clicked(&self) -> bool;
    fn get_text(&self) -> &String;
}
trait ButtonMut
where
    Self: Sized,
{
    type Parent;
    fn click(&mut self);
    fn set_text(&mut self, text: impl Into<String>);
}

struct WindowButton<'a, T: ProbablyMutable> {
    id: usize,
    text: String,
    parent: &'a Window<'a, T>,
}
impl<'a, T: ProbablyMutable> Button<T> for WindowButton<'a, T> {
    type Parent = &'a Window<'a, T>;

    fn new(parent: Self::Parent, id: usize) -> Option<Self> {
        let id = monkey_ffi::make_window_button(id);
        match id {
            0 => None,
            id => Self {
                id,
                text: String::default(),
                parent,
            }
            .into(),
        }
    }
    fn get_id(&self) -> usize {
        self.id
    }

    fn is_clicked(&self) -> bool {
        monkey_ffi::window_button_is_clicked(self.parent.get_id(), self.id)
    }

    fn get_text(&self) -> &String {
        &self.text
    }
}
impl<'a> ButtonMut for WindowButton<'a, Mutable> {
    type Parent = Window<'a, Mutable>;

    fn click(&mut self) {
        monkey_ffi::window_button_click(self.parent.get_id(), self.id)
    }

    fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        monkey_ffi::window_button_set_text(self.parent.get_id(), self.id, &self.text);
    }
}

struct FrameButton<'a, T: ProbablyMutable> {
    id: usize,
    text: String,
    parent: &'a Frame<'a, T>,
}
impl<'a, T: ProbablyMutable> Button<T> for FrameButton<'a, T> {
    type Parent = &'a Frame<'a, T>;

    fn new(parent: Self::Parent, id: usize) -> Option<Self> {
        let id = monkey_ffi::make_frame_button(id);
        match id {
            0 => None,
            id => Self {
                id,
                text: String::default(),
                parent,
            }
            .into(),
        }
    }
    fn get_id(&self) -> usize {
        self.id
    }

    fn is_clicked(&self) -> bool {
        monkey_ffi::frame_button_is_clicked(self.parent.get_id(), self.id)
    }

    fn get_text(&self) -> &String {
        &self.text
    }
}
impl<'a> ButtonMut for FrameButton<'a, Mutable> {
    type Parent = Frame<'a, Mutable>;

    fn click(&mut self) {
        monkey_ffi::frame_button_click(self.parent.get_id(), self.id)
    }

    fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        monkey_ffi::frame_button_set_text(self.parent.get_id(), self.id, &self.text);
    }
}

fn main() {
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
}
