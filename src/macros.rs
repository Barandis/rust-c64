// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

macro_rules! refvec {
    () => (
        $crate::ref_vec::RefVec::new()
    );
    ($item:expr, $n:expr) => (
        $crate::ref_vec::RefVec::with_vec(vec![$item; $n])
    );
    ($($x:expr),+ $(,)?) => (
        $crate::ref_vec::RefVec::with_vec(vec![$($x),+])
    );
}

macro_rules! pin {
    ($number:expr, $name:expr, $mode:expr $(,)?) => {
        $crate::components::pin::Pin::new($number, $name, $mode)
    };
}

macro_rules! pins {
    ($($pin:expr),* $(,)?) => (
        {
            let mut v = refvec![
                pin!(
                    0,
                    $crate::components::device::DUMMY,
                    $crate::components::pin::Mode::Unconnected
                ),
                $(std::rc::Rc::clone(&$pin)),*
            ];
            v.sort_by(|a, b| a.borrow().number().cmp(&b.borrow().number()));
            v
        }
    );
}

#[cfg(test)]
macro_rules! trace {
    ($($pin:expr),* $(,)?) => (
        {
            let v = vec![$(std::rc::Rc::clone(&$pin)),*];
            let t = $crate::components::trace::Trace::new(
                v.clone()
            );
            for p in v.iter() {
                p.borrow_mut().set_trace(std::rc::Rc::clone(&t));
            }
            t
        }
    );
}

macro_rules! new_ref {
    ($obj:expr $(,)?) => {
        std::rc::Rc::new(std::cell::RefCell::new($obj))
    };
}

macro_rules! clone_ref {
    ($obj:expr $(,)?) => {
        std::rc::Rc::clone(&$obj)
    };
}

macro_rules! number {
    ($pin:expr $(,)?) => {
        $pin.borrow().number()
    };
}

macro_rules! name {
    ($pin:expr $(,)?) => {
        $pin.borrow().name()
    };
}

macro_rules! level {
    ($pt:expr $(,)?) => {
        $pt.borrow().level()
    };
}

macro_rules! set_level {
    ($pt:expr, $level:expr $(,)?) => {
        $pt.borrow_mut().set_level($level)
    };
}

macro_rules! high {
    ($pt:expr $(,)?) => {
        $pt.borrow().high()
    };
}

macro_rules! low {
    ($pt:expr $(,)?) => {
        $pt.borrow().low()
    };
}

#[cfg(test)]
macro_rules! floating {
    ($pt:expr $(,)?) => {
        $pt.borrow().floating()
    };
}

macro_rules! set {
    ($($pt:expr),* $(,)?) => (
        $($pt.borrow_mut().set();)*
    );
}

macro_rules! clear {
    ($($pt:expr),* $(,)?) => (
        $($pt.borrow_mut().clear();)*
    );
}

macro_rules! float {
    ($($pt:expr),* $(,)?) => (
        $($pt.borrow_mut().float();)*
    );
}

#[cfg(test)]
macro_rules! toggle {
    ($pt:expr $(,)?) => {
        $pt.borrow_mut().toggle()
    };
}

macro_rules! mode {
    ($pin:expr $(,)?) => {
        $pin.borrow().mode()
    };
}

macro_rules! set_mode {
    ($pin:expr, $mode:expr $(,)?) => {
        $pin.borrow_mut().set_mode($mode)
    };
}

#[cfg(test)]
macro_rules! pull_up {
    ($pt:expr $(,)?) => {
        $pt.borrow_mut().pull_up()
    };
}

#[cfg(test)]
macro_rules! pull_down {
    ($pt:expr $(,)?) => {
        $pt.borrow_mut().pull_down()
    };
}

#[cfg(test)]
macro_rules! pull_off {
    ($pt:expr $(,)?) => {
        $pt.borrow_mut().pull_off()
    };
}

macro_rules! attach {
    ($pin:expr, $obs:expr $(,)?) => {
        $pin.borrow_mut().attach($obs)
    };
}

#[cfg(test)]
macro_rules! detach {
    ($pin:expr $(,)?) => {
        $pin.borrow_mut().detach()
    };
}

macro_rules! attach_to {
    ($device:expr, $($pin:expr),+ $(,)?) => (
        $(attach!($pin, clone_ref!($device)));+
    );
}
