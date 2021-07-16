// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::ops::{Deref, DerefMut};
use std::{cell::RefCell, rc::Rc};

/// A vector with three extra operations on it dealing with shared, internally mutable
/// references.
///
/// The underlying `Vec` doesn't contain items of type `T` itself, but rather items of type
/// `Rc<RefCell<T>>`. This means that the items in the vector can be shared (`Rc`, allows
/// for multiple owners and ensures that the item is not deleted until all ownership has
/// been released) and do not have to convince the compiler that they're following the
/// borrowing rules (`RefCell`, which checks the borrow conditions at runtime instead of at
/// compile time).
///
/// The reason behind this is that in a project of this nature, there is a lot of sharing. A
/// `Pin` needs to be able to be owned and mutated by both the `Device` that it's a part of
/// and the `Trace` that connects it to other pins. The `Device` itself might be mutated by
/// any number of its own pins. And the `Trace` that has more than one output pin connected
/// to it needs to be able to be mutated by all of those output pins.
///
/// Furthermore, there are instances where a device mutates a pin, causing that pin to
/// mutate its trace, causing it to have to check the values of all of its pins (including
/// the one that just mutated it). Hence the pin has already been borrowed mutably, which
/// means it cannot be borrowed again to have its value checked (by the fundamental
/// borrowing rules of Rust, if data is borrowed mutably, it cannot also be borrowed
/// immutably).
///
/// All of this together means that 1) `Pin`s, `Trace`s, and `Device`s all need to be able
/// to be referenced by multiple other structs (hence the need for `Rc`), and 2) it is
/// impossible in some cases to prove to the compiler that borrowing rules are satisfied, so
/// those rules need to instead be checked at runtime (hence the need for `RefCell`).
///
/// The `Rc<RefCell<T>>` mechanism therefore gives a chance to make these things work, but
/// just wrapping all of your `Pin`s with `Rc::new(RefCell::new(pin))` isn't enough. The
/// borrowing rules are still checked. If nothing else changes it just means that, while
/// your program will compile now, it'll just panic at runtime.
///
/// This is a concern because of the nested nature of method calls in the
/// respond-to-an-event kind of mechanism that underlies everything in this project. Say you
/// have a `Pin` named pin, and its connected `Trace` (call it `trace`) changes its level by
/// calling `pin.update()`. Inside that `update` invocation, it'll call `pin.notify()`,
/// which will in turn notify its `Device` (device`) by calling `device.update()`. So at
/// this point, your call stack looks something like this:
///
/// ```text
///     pin calls    device.update()
///     pin calls    pin.notify()
///     trace calls  pin.update()       <-- mutated pin by changing its level
/// ```
/// Now, what if `device.update()` has code that calls `pin.level()` to check the pin's new
/// level? Well, that seems completely logical, but it can't. `trace` still holds a mutable
/// reference to `pin` way down at the bottom of the stack, and until it releases it by
/// having `pin.update()` complete, no other reference can be taken to `pin`. In Rust, a bit
/// of data (`pin` in this case) can have any number of immutable references taken, OR it
/// can have a single mutable reference taken. It cannot have both.
///
/// The situation does not improve merely by having `pin` be an `Rc<RefCell<Pin>>`. `trace`
/// still mutably borrows `pin` (this time with `pin.borrow_mut().update()`, provided by
/// `RefCell` to do the runtime-instead-of-compile-time borrow checking), and then `device`
/// still cannot call `pin.borrow().level()` because even runtime borrow checking has to
/// follow the rules.
///
/// Of course, this is solved by having `pin` be an `Rc<RefCell<Pin>>` AND having `trace` do
/// this instead: `Rc::clone(&pin).borrow_mut().update()`. (The difference in sheer number
/// of characters between this and `pin.update()` is why I use macros.) Now, there's still a
/// mutable reference, but it's to *a cloned reference* of `pin`. `pin` itself still gets
/// updated (it's a clone of a reference to `pin`, not a clone of `pin` itself), but when
/// `device` eventually wants to call `pin.borrow().level()`, it works. The mutable
/// reference was taken from a cloned reference to `pin`, not to `pin` itself, so `pin` at
/// this point has not had any mutable references taken to it.
///
/// So, after 75 lines of comments, how does this `RefVec` work into that? Because sometimes
/// it's hard to get a cloned reference *early* enough. In the above example, `trace` has to
/// be what makes the cloned reference. `device` can't do it in its `update()` method; if it
/// tries to clone `pin` with `Rc::clone(&pin)`, it will be duly informed that it can't take
/// a reference to `pin` because `trace` already has a mutable reference to it. And you
/// can't make a cloned reference without having a reference in the first place (hence
/// `&pin` being passed to `Rc::clone()`).
///
/// So the reference needs to be cloned before mutation happens. Most of the time, this
/// isn't that hard to do. But one case where it can be really hard to do is when you want
/// to use iterators. This is relevant, because there are a lot of places throughout this
/// code where iterators are far and away the best choice. For an easy example, if the CPU
/// wants to read a particular memory address, it'll use `utils::value_to_pins()` on its
/// address pins, which will iterate over all of its address pins and set them to the proper
/// values (with mutated references). This will trigger the pins to mutate the traces
/// they're connected to, and those traces are also connected to the memory's address pins,
/// which will be mutated in turn. The memory will then want to use `utils::pins_to_value()`
/// to iterate over its address pins to regenerate the address.
///
/// The problem is that normal iterators take normal references, possibly mutable ones.
/// `utils::value_to_pins()` *wants* to use `iter_mut()` to iterate over the pins, changing
/// each of them to the correct level. But doing so takes normal mutable references to each
/// pin, and then that pin can no longer be referenced again. The answer is to write our
/// *own* iterator, one which will returned *cloned references* rather than the regular ones
/// that normal iterators deal in.
///
/// So there's the entire point of `RefVec`. It's a vector (thanks to `deref`, which will
/// return a `Vec` to be used in any context that requires a `Vec` and not a `RefVec`) that
/// has an additional type of iterator that internally clones references, so the simple act
/// of creating an iterator doesn't mess everything up. It has a couple other new methods -
/// `get_ref()` is like `get` except it returns a cloned reference, and a `clone()`
/// implementation that will return a new `RefVec` of cloned references to all of the
/// original's items.
pub struct RefVec<T>(Vec<Rc<RefCell<T>>>);

/// Here is the iterator itself. It calls `Rc::clone()` on each item referencd in the
/// underlying vector and returns that instead of a plain reference.
pub struct RefIter<'a, T>(&'a [Rc<RefCell<T>>]);

impl<T> RefVec<T> {
    /// Creates a new, empty `RefVec`.
    pub const fn new() -> RefVec<T> {
        RefVec(Vec::new())
    }

    /// Creates a new `RefVec` containing all of the items in the supplied vector. Note that
    /// it does not create cloned references to these items; it's expected that the vector
    /// already contains cloned references.
    pub const fn with_vec(v: Vec<Rc<RefCell<T>>>) -> RefVec<T> {
        RefVec(v)
    }

    /// Returns a cloned reference of an item in the vector.
    pub fn get_ref(&self, index: usize) -> Rc<RefCell<T>> {
        Rc::clone(&self[index])
    }

    /// Returns an iterator that itself returns cloned references to all of the underlying
    /// items.
    pub fn iter_ref(&self) -> RefIter<'_, T> {
        RefIter(self.0.as_slice())
    }
}

impl<'a, T> Iterator for RefIter<'a, T> {
    type Item = Rc<RefCell<T>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.get(0) {
            Some(item) => {
                self.0 = &self.0[1..];
                Some(Rc::clone(item))
            }
            None => None,
        }
    }
}

impl<T> Clone for RefVec<T> {
    /// Returns a clone of the current `RefVec`. This clone will contain cloned references
    /// to each of the references in the original vector.
    fn clone(&self) -> Self {
        RefVec(
            self.0
                .iter()
                .map(|pin| Rc::clone(pin))
                .collect::<Vec<Rc<RefCell<T>>>>(),
        )
    }
}

impl<T> Deref for RefVec<T> {
    type Target = Vec<Rc<RefCell<T>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for RefVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
