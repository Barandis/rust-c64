// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::{cell::RefCell, cmp::Ordering, fmt::Debug, rc::Rc};

use super::pin::{Mode, PinRef};

/// A convenience alias for a shared internally-mutable reference to a Trace, so we don't
/// have to type all those angle brackets.
pub type TraceRef = Rc<RefCell<Trace>>;

/// A printed-circuit board trace that connects two or more pins.
///
/// A trace is designed primarily to have its level modified by a connected output pin.
/// However, the level can also be set directly (this is often useful in testing and
/// debugging). When a trace's level is set directly, its actual value is chosen according
/// to the following rules:
///
/// 1. If the trace has at least one output pin connected to it that has a level, the trace
///    takes on the maximum level among all of its connected output pins.
/// 2. If the value being set is `None`: a. If the trace has been pulled up, its value is
///    1.0. b. If the trace has been pulled down, its value is 0.0. c. Its value is `None`.
/// 3. The trace takes on the set value.
///
/// If a trace is set by a pin (either by an output pin changing values or by an unconnected
/// pin mode-changing into an output pin), then the value is simply set *unless* the value
/// it's being set to is `None`. In that case the same rules as direct setting apply.
///
/// A change in the level of the trace will be propagated to any input pins connected to the
/// trace. When this happens, the observers of all of those input pins are notified of the
/// change.
pub struct Trace {
    /// A list of all of the pins that are connected to this trace.
    pins: Vec<PinRef>,

    /// The level that the trace will take if its level is set to `None` and there are no
    /// output pins with levels that will override this. This value is set by `pull_up`,
    /// `pull_down`, and `pull_off`.
    float: Option<f64>,

    /// The level of the trace. If the trace has no level (i.e., it has no output pins with
    /// levels and has had its own level set to `None`), this will be `None`.
    level: Option<f64>,
}

impl Trace {
    /// Creates a new trace from a vector of pins that are connected to it and returns a
    /// shared, internally mutable reference to it. Its initial level will depend on the
    /// levels of the output pins in that vector (if there are none, the trace's level will
    /// be `None`). It's initial float value will be `None` (i.e., not pulled up or down).
    pub fn new(pins: Vec<PinRef>) -> TraceRef {
        Rc::new(RefCell::new(Trace {
            pins,
            float: None,
            level: None,
        }))
    }

    /// Calculates what the level of the trace should be based on the value it's being set
    /// to, all of its output pins, and whether or not the value is being set by a pin or
    /// directly.
    ///
    /// Essentially, if there is an output pin that has a level, then the new level this
    /// method returns will be equal to the maximum level of all of its output pins (plus
    /// the passed-in level, if `from_pin` is `true`). If there are no output pins with
    /// levels, the passed-in level will be returned, unless that level is `None`, in which
    /// case this traces float value will be returned.
    ///
    /// A reasonable question would be "why pass in the level when it's just coming from an
    /// output pin anyway?" The answer is that this method is often called as a consequence
    /// of the level of an output pin changing. To make that change, a mutable reference to
    /// the pin will have had to have been borrowed. Since that's the case, we can't take
    /// references to that pin out of the vector of pins...that would be borrowing a
    /// reference to a value that has already been borrowed mutable, and that's a no-no.
    /// Since this is a private method only used internally, this doesn't create any real
    /// complexity issues.
    fn calculate(&self, level: Option<f64>, from_pin: bool) -> Option<f64> {
        match self
            .pins
            .iter()
            .filter(|&pin| match pin.try_borrow() {
                Ok(p) => p.mode() == Mode::Output && !p.floating(),
                Err(_) => false,
            })
            .max_by(|x, y| {
                // `unwrap` is fine here because anything with a `None` level has already
                // been filtered out
                match x
                    .borrow()
                    .level()
                    .unwrap()
                    .partial_cmp(&y.borrow().level().unwrap())
                {
                    Some(order) => order,
                    // This isn't actually a possibility - all `None` values have already
                    // been filtered out - but we have to keep the compiler happy.
                    None => Ordering::Less,
                }
            }) {
            Some(maxpin) => {
                // `unwrap` is fine here because anything with a `None` level has already
                // been filtered out
                let plevel = maxpin.borrow().level().unwrap();
                if from_pin {
                    match level {
                        Some(ilevel) => {
                            if ilevel > plevel {
                                Some(ilevel)
                            } else {
                                Some(plevel)
                            }
                        }
                        None => Some(plevel),
                    }
                } else {
                    Some(plevel)
                }
            }
            None => match level {
                Some(_) => level,
                None => self.float,
            },
        }
    }

    /// Returns the level of the trace. This can be `None` if no output pins are driving the
    /// trace.
    pub fn level(&self) -> Option<f64> {
        self.level
    }

    /// Sets a new level for the trace. This is a direct setting of the trace and is not
    /// considered to have come from a pin (pins use `update` instead). It will be
    /// overridden if there is an output pin connected to the trace that has a non-`None`
    /// level.
    pub fn set_level(&mut self, level: Option<f64>) {
        self.level = self.calculate(level, false);
        for pin in self.pins.iter_mut() {
            pin.borrow_mut().update(self.level);
        }
    }

    /// Determines whether the trace's level is high. This conventionally means a level of
    /// `1.0`, but any value of `0.5` or higher will register as high.
    pub fn high(&self) -> bool {
        match self.level {
            None => false,
            Some(n) => n >= 0.5,
        }
    }

    /// Determines whether the trace's level is low. This conventionally means a level of
    /// `0.0`, but any value less than `0.5` will register as low.
    pub fn low(&self) -> bool {
        match self.level {
            None => false,
            Some(n) => n < 0.5,
        }
    }

    /// Determines whether the trace's level is floating. This means it has no level at all,
    /// normally indicative of a trace with no output pins with levels.
    pub fn floating(&self) -> bool {
        match self.level {
            None => true,
            _ => false,
        }
    }

    /// Sets the traces's level to high (`Some(1.0)`). This will have no effect if the trace
    /// has an output pin connected to it with a non-`None` level.
    pub fn set(&mut self) {
        self.set_level(Some(1.0));
    }

    /// Sets the traces's level to low (`Some(0.0)`). This will have no effect if the trace
    /// has an output pin connected to it with a non-`None` level.
    pub fn clear(&mut self) {
        self.set_level(Some(0.0));
    }

    /// Sets the traces's level to floating (`None`). This will have no effect if the trace
    /// has an output pin connected to it with a non-`None` level. If that's not the case
    /// but the trace is being pulled up or down, that will override this `None` value.
    pub fn float(&mut self) {
        self.set_level(None);
    }

    /// Toggles the pin's value. If the pin was high (`0.5` or higher), its new level will
    /// become `Some(0.0)`, and vice versa. This function has no effect on pins with a level
    /// of `None`. It also has no effect if the trace has non-`None` levelled output pins
    /// connected to it.
    pub fn toggle(&mut self) {
        if self.high() {
            self.clear();
        } else if self.low() {
            self.set();
        }
    }

    /// Sets a new level for the trace. This method is assumed to have been called by a pin,
    /// so its visibilty is limited to the components module. It *will* factor into level
    /// calculations alongside other connected output pins, and it will notify observers of
    /// input pins that it connects to.
    pub(super) fn update(&mut self, level: Option<f64>) {
        self.level = self.calculate(level, true);
        for pin in self.pins.iter() {
            if let Ok(mut p) = pin.try_borrow_mut() {
                p.update(level);
            }
        }
    }

    /// Sets the trace to be pulled up. If a trace is pulled up, setting it to a level of
    /// `None` will cause it to instead be set to `Some(1.0)`. This emulates traces that are
    /// connected to pull-up resistors connected to the power supply that are intended to
    /// make the trace level high unless another output pin is driving it.
    pub fn pull_up(&mut self) {
        self.float = Some(1.0);
        self.set_level(self.level);
    }

    /// Sets the trace to be pulled down. If a trace is pulled down, setting it to a level
    /// of `None` will cause it to instead be set to `Some(0.0)`. This emulates traces that
    /// are connected to pull-down resistors connected to ground that are intended to make
    /// the trace level high unless another output pin is driving it.
    pub fn pull_down(&mut self) {
        self.float = Some(0.0);
        self.set_level(self.level);
    }

    /// Removes any pull-up or pull-down status for the trace. The trace will take levels
    /// normally, taking on the level `None` if it is set to `None`.
    pub fn pull_off(&mut self) {
        self.float = None;
        self.set_level(self.level);
    }

    /// Connects a pin to this trace. This will only actually happen if the pin is not
    /// already connected to a trace. The trace's value will then be recalculated based on
    /// the new pin's level and mode.
    pub fn add_pin(&mut self, pin: PinRef) {
        if !pin.borrow().connected() {
            self.pins.push(pin);
            self.set_level(self.level);
        }
    }

    /// Connects a list of pins to this trace. Each individual pin is checked to see if it's
    /// already connected to a trace, and if it is, that pin is *not* connected to this one.
    /// The trace's value is recalculated based on these new pins' levels and modes.
    pub fn add_pins(&mut self, pins: Vec<PinRef>) {
        for pin in pins.into_iter() {
            self.add_pin(pin);
        }
    }
}

impl Debug for Trace {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let alt = f.alternate();
        let mut str = String::from("Trace(");
        if alt {
            str.push_str("\n    ");
        }
        str.push_str(format!("level = {:?}", self.level).as_str());
        if alt {
            str.push_str("\n    ");
        } else {
            str.push_str(", ");
        }
        str.push_str(format!("float = {:?}", self.float).as_str());
        if alt {
            str.push('\n');
        }
        str.push(')');
        write!(f, "{}", str)
    }
}

#[cfg(test)]
mod test {
    use crate::components::device::{Device, LevelChangeEvent};

    use super::*;
    use crate::components::pin::Mode::{Bidirectional, Input, Output, Unconnected};

    // Testing (as opposed to using) Devices is weird; see the long-winded explanation
    // in pin.rs for why.
    struct TestDevice {
        count: usize,
        level: Option<f64>,
    }

    impl TestDevice {
        fn new() -> TestDevice {
            TestDevice {
                count: 0,
                level: None,
            }
        }
    }

    impl Device for TestDevice {
        fn update(&mut self, event: &LevelChangeEvent) {
            self.count += 1;
            self.level = event.2;
        }

        fn pins(&self) -> Vec<PinRef> {
            Vec::new()
        }

        fn registers(&self) -> Vec<u8> {
            Vec::new()
        }
    }

    #[test]
    fn no_add_twice() {
        let p = pin!(1, "A", Input);
        let t = trace!(p, p);

        let d = Rc::new(RefCell::new(TestDevice::new()));
        let tested = Rc::clone(&d);
        attach!(p, d);

        set!(t);
        assert_eq!(tested.borrow().count, 1);
    }

    #[test]
    fn level_direct_unconnected() {
        let t = trace!();

        set!(t);
        assert!(high!(t));
        assert!(!low!(t));
        assert!(!floating!(t));

        clear!(t);
        assert!(!high!(t));
        assert!(low!(t));
        assert!(!floating!(t));

        float!(t);
        assert!(!high!(t));
        assert!(!low!(t));
        assert!(floating!(t));

        set_level!(t, Some(-0.25));
        assert_eq!(level!(t).unwrap(), -0.25);
    }

    #[test]
    fn level_direct_input() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);

        set!(t);
        assert!(high!(t));
        clear!(t);
        assert!(low!(t));
        float!(t);
        assert!(floating!(t));
        set_level!(t, Some(-0.25));
        assert_eq!(level!(t).unwrap(), -0.25);
    }

    #[test]
    fn level_direct_output_high() {
        let p1 = pin!(1, "HI", Output);
        let p2 = pin!(2, "LO", Output);
        let t = trace!(p1, p2);

        set!(p1);
        clear!(p2);

        set!(t);
        assert!(high!(t));
        clear!(t);
        assert!(high!(t));
        float!(t);
        assert!(high!(t));
        set_level!(t, Some(0.25));
        assert!(high!(t));
    }

    #[test]
    fn level_direct_output_low() {
        let p1 = pin!(1, "HI", Output);
        let p2 = pin!(2, "LO", Output);
        let t = trace!(p1, p2);

        clear!(p1);
        clear!(p2);

        set!(t);
        assert!(low!(t));
        clear!(t);
        assert!(low!(t));
        float!(t);
        assert!(low!(t));
        set_level!(t, Some(0.75));
        assert!(low!(t));
    }

    #[test]
    fn level_direct_output_float() {
        let p1 = pin!(1, "HI", Output);
        let p2 = pin!(2, "LO", Output);
        let t = trace!(p1, p2);

        float!(p1);
        float!(p2);

        set!(t);
        assert!(high!(t));
        clear!(t);
        assert!(low!(t));
        float!(t);
        assert!(floating!(t));
        set_level!(t, Some(0.25));
        assert_eq!(level!(t).unwrap(), 0.25);
    }

    #[test]
    fn level_pin_unconnected() {
        let p = pin!(1, "A", Unconnected);
        let t = trace!(p);
        clear!(t);

        set!(p);
        assert!(low!(t));
        assert!(high!(p));
    }

    #[test]
    fn level_pin_input() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);
        clear!(t);

        set!(p);
        assert!(low!(t));
        assert!(low!(p));
    }

    #[test]
    fn level_pin_ouput() {
        let p = pin!(1, "A", Output);
        let t = trace!(p);
        clear!(t);

        set!(p);
        assert!(high!(t));
        assert!(high!(p));
    }

    #[test]
    fn level_pin_bidirectional() {
        let p = pin!(1, "A", Bidirectional);
        let t = trace!(p);
        clear!(t);

        set!(p);
        assert!(high!(t));
        assert!(high!(p));

        float!(t);
        assert!(floating!(t));
        assert!(floating!(p));
    }

    #[test]
    fn level_pin_outputs_high() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        let p3 = pin!(3, "C", Output);
        let t = trace!(p1, p2, p3);

        set!(p2);
        set!(p3);

        clear!(p1);
        assert!(high!(t));
    }

    #[test]
    fn level_pin_outputs_low() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        let p3 = pin!(3, "C", Output);
        let t = trace!(p1, p2, p3);

        clear!(p2);
        clear!(p3);

        set!(p1);
        assert!(high!(t));
    }

    #[test]
    fn pull_up_initial() {
        let t = trace!();
        pull_up!(t);
        assert!(high!(t));
    }

    #[test]
    fn pull_up_input() {
        let p = pin!(1, "A", Output);
        let t = trace!(p);
        pull_up!(t);

        clear!(p);
        assert!(low!(t));
        set_mode!(p, Input);
        assert!(high!(t));
    }

    #[test]
    fn pull_up_output_none() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);
        pull_up!(t);
        assert!(high!(t));
    }

    #[test]
    fn pull_up_output_high() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        set!(p1);
        clear!(p2);
        let t = trace!(p1, p2);
        pull_up!(t);
        assert!(high!(t));
    }

    #[test]
    fn pull_up_output_low() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        clear!(p1);
        clear!(p2);
        let t = trace!(p1, p2);
        pull_up!(t);
        assert!(low!(t));
    }

    #[test]
    fn pull_up_output_floating() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        float!(p1);
        float!(p2);
        let t = trace!(p1, p2);
        pull_up!(t);
        assert!(high!(t));
    }

    #[test]
    fn pull_down_initial() {
        let t = trace!();
        pull_down!(t);
        assert!(low!(t));
    }

    #[test]
    fn pull_down_input() {
        let p = pin!(1, "A", Output);
        let t = trace!(p);
        pull_down!(t);

        set!(p);
        assert!(high!(t));
        set_mode!(p, Input);
        assert!(low!(t));
    }

    #[test]
    fn pull_down_output_none() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);
        pull_down!(t);
        assert!(low!(t));
    }

    #[test]
    fn pull_down_output_high() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        set!(p1);
        clear!(p2);
        let t = trace!(p1, p2);
        pull_down!(t);
        assert!(high!(t));
    }

    #[test]
    fn pull_down_output_low() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        clear!(p1);
        clear!(p2);
        let t = trace!(p1, p2);
        pull_down!(t);
        assert!(low!(t));
    }

    #[test]
    fn pull_down_output_floating() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        float!(p1);
        float!(p2);
        let t = trace!(p1, p2);
        pull_down!(t);
        assert!(low!(t));
    }

    #[test]
    fn pull_off_initial() {
        let t = trace!();
        pull_off!(t);
        assert!(floating!(t));
    }

    #[test]
    fn pull_off_input() {
        let p = pin!(1, "A", Output);
        let t = trace!(p);
        pull_off!(t);

        set!(p);
        assert!(high!(t));
        set_mode!(p, Input);
        assert!(floating!(t));
    }

    #[test]
    fn pull_off_output_none() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);
        pull_off!(t);
        assert!(floating!(t));
    }

    #[test]
    fn pull_off_output_high() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        set!(p1);
        clear!(p2);
        let t = trace!(p1, p2);
        pull_off!(t);
        assert!(high!(t));
    }

    #[test]
    fn pull_off_output_low() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        clear!(p1);
        clear!(p2);
        let t = trace!(p1, p2);
        pull_off!(t);
        assert!(low!(t));
    }

    #[test]
    fn pull_off_output_floating() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Output);
        float!(p1);
        float!(p2);
        let t = trace!(p1, p2);
        pull_off!(t);
        assert!(floating!(t));
    }
}
