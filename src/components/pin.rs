// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use std::{
    cell::RefCell,
    fmt::{Debug, Error, Formatter},
    rc::Rc,
};

use super::{
    device::{DeviceRef, LevelChange},
    trace::TraceRef,
};

/// A convenience alias for a shared internally-mutable reference to a Pin, so we don't have
/// to type all those angle brackets.
pub type PinRef = Rc<RefCell<Pin>>;

/// The direction through which data can flow through a pin.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Indicates that the pin is not connected. It will not accept data from a trace and
    /// will not pass data to a trace. Event listeners will not be fired.
    Unconnected,

    /// Indicates that the pin is used for input from a trace to a chip or a port. Setting
    /// the level of this pin will not have an effect if it's connected to a trace, and a
    /// changed level on the connected trace will be propagated to the pin, firing its
    /// listeners.
    Input,

    /// Indicates that the pin is used for output from a chip or port to a trace. Setting
    /// the level of this pin will also set the level of the trace, and a change to the
    /// traces level will have no effect unless it's `None`.
    Output,

    /// Indicates that the pin is used for both input to *and* output from a chip or a port.
    /// Setting the level of a connected trace will change the level of this pin (and invoke
    /// listeners), and setting the level of the pin will change the level of the connected
    /// trace. Note that this is for pins that need to handle both input and output
    /// simultaneously; a pin that is sometimes an input pin and sometimes an output pin
    /// (like pins connected to data bus lines) should have its mode changed to whatever is
    /// appropriate at the time.
    Bidirectional,
}

/// A pin on an IC package or a port.
///
/// This is the sole interface between these devices and the outside world. Pins have a
/// direction, which indicates whether they're used by their chip/port for input, output,
/// both, or neither; and a level, which is the signal present on them. In digital circuits,
/// this is generally 0 or 1, though there's no reason a pin can't work with analog signals
/// (and thus have any level at all). For that reason, the level is implemented as an f64,
/// though all of the level-setting functions other than set_level set either 1 or 0.
///
/// Pins may also be pulled up or down, which defines what level they have if a level isn't
/// given to them. This emulates the internal pull-ups and pull-downs that some chips have
/// (such as the port pins on a 6526 CIA). If no level is given to them and they have no
/// pull-up or pull-down, then their level will be `None`. This can be used to represent,
/// e.g., a high-impedance state that cuts the pin off from its circuit.
///
/// A pin maintains a list of observers that will be notified each time the pin's value
/// changes, as long as the pin is in Input or Bidirectional mode. Generally a pin will have
/// only one observer - the device to which it's attached.
pub struct Pin {
    /// The pin number. This is normally defined in chip or port literature.
    number: usize,

    /// The pin name. Again, thjis is normally defined in chip or port literature.
    name: &'static str,

    /// The level that the pin will take if its level is set to `None`. This value is set
    /// by `pull_up`, `pull_down`, and `pull_off`.
    float: Option<f64>,

    /// The level of the pin. If the pin has no level (i.e., it's disconnected or in a hi-Z
    /// state), this will be `None`.
    level: Option<f64>,

    /// The trace to which this pin is connected. Will be `None` if the pin has not been
    /// connected to a trace. Once a trace has been connected, there is no way to disconnect
    /// it (physical pins don't generally change their traces either).
    trace: Option<TraceRef>,

    /// The mode of the pin, a description of which direction data is flowing through it.
    mode: Mode,

    /// A list of observers that will have their `update` methods called when this pin
    /// changes level.
    device: Option<DeviceRef>,
}

/// Normalizes a level, returning that level unless it is `None`. If it *is* `None`, the
/// `float` parameter will be returned instead.
fn normalize(level: Option<f64>, float: Option<f64>) -> Option<f64> {
    match level {
        None => float,
        _ => level,
    }
}

impl Pin {
    /// Creates a new pin and returns a shared, internally mutable reference to it. The pin
    /// will be in the supplied state with a level and float level of `None`.
    pub fn new(number: usize, name: &'static str, mode: Mode) -> PinRef {
        Rc::new(RefCell::new(Pin {
            number,
            name,
            mode,
            float: None,
            level: None,
            trace: None,
            device: None,
        }))
    }

    /// Sets the pin's connected trace. This trace must be wrapped in an `Rc`'d `RefCell`
    /// because both this pin and the trace itself need to be able to change the trace's
    /// level. After this function is called, the trace can never again be `None`.
    pub fn set_trace(&mut self, trace: TraceRef) {
        self.trace = Some(trace);
    }

    /// Returns the pin number.
    pub fn number(&self) -> usize {
        self.number
    }

    /// Returns the pin name.
    pub fn name(&self) -> &str {
        self.name
    }

    /// Returns the level of the pin. This can be `None` if the pin is in a hi-Z state.
    pub fn level(&self) -> Option<f64> {
        self.level
    }

    /// Sets the level of the pin. The supplied value does not automatically become the
    /// pin's level; a pin in `Input` mode will ignore a level set by this function.
    pub fn set_level(&mut self, level: Option<f64>) {
        self.level = match &self.trace {
            None => normalize(level, self.float),
            Some(trace) => match self.mode {
                Mode::Unconnected => normalize(level, self.float),
                Mode::Input => self.level,
                Mode::Output | Mode::Bidirectional => {
                    let normalized = normalize(level, self.float);
                    trace.borrow_mut().update(normalized);
                    normalized
                }
            },
        }
    }

    /// Determines whether the pin's level is high. This conventionally means a level of
    /// `1.0`, but any value of `0.5` or higher will register as high.
    pub fn high(&self) -> bool {
        match self.level {
            None => false,
            Some(n) => n >= 0.5,
        }
    }

    /// Determines whether the pin's level is low. This conventionally means a level of
    /// `0.0`, but any value less than `0.5` will register as low.
    pub fn low(&self) -> bool {
        match self.level {
            None => false,
            Some(n) => n < 0.5,
        }
    }

    /// Determines whether the pin's level is floating. This means it has no level at all,
    /// normally indicative of a disconnected or hi-Z state.
    pub fn floating(&self) -> bool {
        match self.level {
            None => true,
            _ => false,
        }
    }

    /// Sets the pin's level to high (`Some(1.0)`).
    pub fn set(&mut self) {
        self.set_level(Some(1.0));
    }

    /// Sets the pin's level to low (`Some(0.0)`).
    pub fn clear(&mut self) {
        self.set_level(Some(0.0));
    }

    /// Sets the pin's level to floating (`None`).
    pub fn float(&mut self) {
        self.set_level(None);
    }

    /// Toggles the pin's value. If the pin was high (`0.5` or higher), its new level will
    /// become `Some(0.0)`, and vice versa. This function has no effect on pins with a level
    /// of `None`.
    pub fn toggle(&mut self) {
        if self.high() {
            self.clear();
        } else if self.low() {
            self.set();
        }
    }

    /// Updates the pin's value if it is an input pin (mode `Input` or `Bidirectional`).
    /// This will notify observers of the pin if its level actually changes (it's not being
    /// set to the same level it aleady had).
    ///
    /// This method should only be called by a connected trace, so its visibility is limited
    /// to the components module.
    pub(super) fn update(&mut self, level: Option<f64>) {
        let old_level = self.level;
        let new_level = normalize(level, self.float);
        if self.input() && new_level != old_level {
            self.level = new_level;
            self.notify();
        }
    }

    /// Returns the pin's current mode.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Sets the pin's mode. This can, depending on the new and old modes, also update the
    /// connected trace. For example, if a pin changes to an output mode (`Output` or
    /// `Bidirectional`), its level will propagate to the connected trace. A pin of mode
    /// `Input` will change its own value to match that of its connected trace. If that pin
    /// was an output pin prior to this change, then the trace's level will be recalculated
    /// based on having one less output pin connected to it.
    pub fn set_mode(&mut self, mode: Mode) {
        let old_mode = self.mode;
        let old_level = self.level;
        self.mode = mode;

        if let Some(trace) = &self.trace {
            match mode {
                Mode::Output | Mode::Bidirectional => trace.borrow_mut().update(self.level),
                Mode::Input | Mode::Unconnected => {
                    if mode == Mode::Input {
                        self.level = normalize(trace.borrow().level(), self.float);
                    }
                    if old_level.is_some()
                        && (old_mode == Mode::Output || old_mode == Mode::Bidirectional)
                    {
                        trace.borrow_mut().update(None);
                    }
                }
            }
        }
    }

    /// Determines whether the pin is an input pin (mode `Input` or `Bidirectional`).
    pub fn input(&self) -> bool {
        match self.mode {
            Mode::Input | Mode::Bidirectional => true,
            _ => false,
        }
    }

    /// Determines whether the pin is an output pin (mode `Output` or `Bidirectional`).
    pub fn output(&self) -> bool {
        match self.mode {
            Mode::Output | Mode::Bidirectional => true,
            _ => false,
        }
    }

    /// Sets the pin to be pulled up. If a pin is pulled up, setting it to a level of `None`
    /// will cause it to instead be set to `Some(1.0)`. This emulates pins that are
    /// internally pulled up, like the parallel port pins on the 6526 CIA.
    pub fn pull_up(&mut self) {
        self.float = Some(1.0);
        self.level = normalize(self.level, self.float);
    }

    /// Sets the pin to be pulled down. If a pin is pulled down, setting it to a level of
    /// `None` will cause it instead to be set to `Some(0.0)`. This emulates pins that are
    /// internally pulled down.
    pub fn pull_down(&mut self) {
        self.float = Some(0.0);
        self.level = normalize(self.level, self.float);
    }

    /// Removes any pull-up or pull-down status for the pin. The pin will take levels
    /// normally, taking on the level `None` if it is set to `None`.
    pub fn pull_off(&mut self) {
        self.float = None;
        self.level = normalize(self.level, self.float);
    }

    /// Determines whether the pin has a connected trace. This is a convenience function
    /// used by `Trace` to ensure that it can only connect to a pin that doesn't already
    /// have a trace connected.
    pub fn connected(&self) -> bool {
        match self.trace {
            None => false,
            _ => true,
        }
    }

    /// Attaches an observer to this pin. In reality every pin should have one observer
    /// because each pin belongs to only one device, but this will allow a pin to be
    /// observed in testing or debugging as well.
    pub fn attach(&mut self, device: DeviceRef) {
        self.device = Some(device);
    }

    /// Detaches an observer from this pin. The observer is found by its `id` method and the
    /// first one with the same id as the supplied observer is removed.
    ///
    /// Each pin should have one observer (the device it belongs to) and that observer
    /// should have an id of 0. Moreover, it should not ever have to be detached. This
    /// method allows there to be temporary debugging/testing observers that can be attached
    /// and detached at will.
    pub fn detach(&mut self) {
        self.device = None;
    }

    /// Notifies this pin's observers of a change to its
    fn notify(&self) {
        let pin = Rc::new(RefCell::new(self));
        let event = &LevelChange(pin);
        for ob in self.device.iter() {
            ob.borrow_mut().update(event);
        }
    }
}

impl Debug for Pin {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        let alt = f.alternate();
        let mut str = String::from("Pin(");
        if alt {
            str.push_str("\n    ");
        }
        str.push_str(format!("number = {:?}", self.number).as_str());
        if alt {
            str.push_str("\n    ");
        }
        str.push_str(format!("name   = {:?}", self.name).as_str());
        if alt {
            str.push_str("\n    ");
        }
        str.push_str(format!("level  = {:?}", self.level).as_str());
        if alt {
            str.push_str("\n    ");
        } else {
            str.push_str(", ");
        }
        str.push_str(format!("float  = {:?}", self.float).as_str());
        if alt {
            str.push_str("\n    ");
        } else {
            str.push_str(", ");
        }
        str.push_str(format!("mode   = {:?}", self.mode).as_str());
        if alt {
            str.push('\n');
        }
        str.push(')');
        write!(f, "{}", str)
    }
}

#[cfg(test)]
mod test {
    use crate::components::device::Device;
    use crate::ref_vec::RefVec;

    use super::Mode::{Bidirectional, Input, Output, Unconnected};
    use super::*;

    #[test]
    fn has_number() {
        let pin = Pin::new(1, "A", Unconnected);
        assert_eq!(pin.borrow().number(), 1);
    }

    #[test]
    fn has_name() {
        let pin = Pin::new(1, "A", Unconnected);
        assert_eq!(pin.borrow().name(), "A");
    }

    #[test]
    fn mode_initial() {
        let p1 = Pin::new(1, "A", Unconnected);
        let p2 = Pin::new(2, "B", Input);
        let p3 = Pin::new(3, "C", Output);
        let p4 = Pin::new(4, "D", Bidirectional);

        assert_eq!(p1.borrow().mode(), Unconnected);
        assert!(!p1.borrow().input());
        assert!(!p1.borrow().output());

        assert_eq!(p2.borrow().mode(), Input);
        assert!(p2.borrow().input());
        assert!(!p2.borrow().output());

        assert_eq!(p3.borrow().mode(), Output);
        assert!(!p3.borrow().input());
        assert!(p3.borrow().output());

        assert_eq!(p4.borrow().mode(), Bidirectional);
        assert!(p4.borrow().input());
        assert!(p4.borrow().output());
    }

    #[test]
    fn mode_change() {
        let p = Pin::new(1, "A", Unconnected);
        assert_eq!(p.borrow().mode(), Unconnected);

        p.borrow_mut().set_mode(Input);
        assert_eq!(p.borrow().mode(), Input);

        p.borrow_mut().set_mode(Output);
        assert_eq!(p.borrow().mode(), Output);

        p.borrow_mut().set_mode(Bidirectional);
        assert_eq!(p.borrow().mode(), Bidirectional);
    }

    #[test]
    fn mode_out_to_in() {
        let p1 = pin!(1, "A", Output);
        let p2 = pin!(2, "B", Input);
        let t = trace!(p1, p2);

        set!(p1);
        assert!(high!(t));
        set_mode!(p1, Input);
        assert!(floating!(t));
    }

    #[test]
    fn mode_bidi_to_in() {
        let p = pin!(1, "A", Bidirectional);
        let t = trace!(p, pin!(2, "B", Input));

        set!(p);
        assert!(high!(t));
        set_mode!(p, Input);
        assert!(floating!(t));
    }

    #[test]
    fn mode_unc_to_in() {
        let p = pin!(1, "A", Unconnected);
        let t = trace!(p, pin!(2, "B", Input));

        set!(p);
        assert!(floating!(t));
        set_mode!(p, Input);
        assert!(floating!(t));
    }

    #[test]
    fn mode_bidi_to_out() {
        let p = pin!(1, "A", Bidirectional);
        let t = trace!(p);

        set!(p);
        assert!(high!(t));
        set_mode!(p, Output);
        assert!(high!(t));
    }

    #[test]
    fn mode_unc_to_out() {
        let p = pin!(1, "A", Unconnected);
        let t = trace!(p);

        set!(p);
        assert!(floating!(t));
        set_mode!(p, Output);
        assert!(high!(t));
    }

    #[test]
    fn mode_in_to_unc() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);

        set!(t);
        assert!(high!(p));
        set_mode!(p, Unconnected);
        assert!(high!(t));
        assert!(high!(p));
    }

    #[test]
    fn level_no_trace() {
        let p = Pin::new(1, "A", Input);
        assert!(p.borrow().level().is_none());
        assert!(!p.borrow().high());
        assert!(!p.borrow().low());
        assert!(p.borrow().floating());

        p.borrow_mut().set_level(Some(1.0));
        assert_eq!(p.borrow().level().unwrap(), 1.0);
        assert!(p.borrow().high());
        assert!(!p.borrow().low());
        assert!(!p.borrow().floating());

        p.borrow_mut().set_level(Some(0.0));
        assert_eq!(p.borrow().level().unwrap(), 0.0);
        assert!(!p.borrow().high());
        assert!(p.borrow().low());
        assert!(!p.borrow().floating());

        p.borrow_mut().set_level(Some(0.25));
        assert_eq!(p.borrow().level().unwrap(), 0.25);
        assert!(!p.borrow().high());
        assert!(p.borrow().low());
        assert!(!p.borrow().floating());
    }

    #[test]
    fn level_update_no_trace() {
        let p = Pin::new(1, "A", Input);
        p.borrow_mut().set();
        p.borrow_mut().update(None);
        assert!(p.borrow().level().is_none());
    }

    #[test]
    fn level_unconnected() {
        let p = pin!(1, "A", Unconnected);
        let t = trace!(p);

        set!(t);
        assert!(floating!(p));
        assert!(high!(t));

        set!(p);
        assert!(high!(p));
        assert!(high!(t));

        clear!(p);
        assert!(low!(p));
        assert!(high!(t));

        set_level!(p, Some(0.25));
        assert_eq!(level!(p).unwrap(), 0.25);
        assert!(high!(t));

        float!(p);
        assert!(floating!(p));
        assert!(high!(t));
    }

    #[test]
    fn level_input() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);

        set!(t);
        assert!(high!(p));
        assert!(high!(t));

        set!(p);
        assert!(high!(p));
        assert!(high!(t));

        clear!(p);
        assert!(high!(p));
        assert!(high!(t));

        set_level!(p, Some(0.25));
        assert_eq!(level!(p).unwrap(), 1.0);
        assert!(high!(t));

        float!(p);
        assert!(high!(p));
        assert!(high!(t));
    }

    #[test]
    fn level_output() {
        let p = pin!(1, "A", Output);
        let t = trace!(p);

        set!(t);
        assert!(floating!(p));
        assert!(high!(t));

        set!(p);
        assert!(high!(p));
        assert!(high!(t));

        clear!(p);
        assert!(low!(p));
        assert!(low!(t));

        set_level!(p, Some(0.25));
        assert_eq!(level!(p).unwrap(), 0.25);
        assert_eq!(level!(t).unwrap(), 0.25);

        float!(p);
        assert!(floating!(p));
        assert!(floating!(t));
    }

    #[test]
    fn level_bidirectional() {
        let p = pin!(1, "A", Bidirectional);
        let t = trace!(p);

        set!(t);
        assert!(high!(p));
        assert!(high!(t));

        set!(p);
        assert!(high!(p));
        assert!(high!(t));

        clear!(p);
        assert!(low!(p));
        assert!(low!(t));

        set_level!(t, Some(0.25));
        assert_eq!(level!(p).unwrap(), 0.25);
        assert_eq!(level!(t).unwrap(), 0.25);

        float!(p);
        assert!(floating!(p));
        assert!(floating!(t));
    }

    #[test]
    fn level_toggle_high() {
        let p = pin!(1, "A", Unconnected);
        clear!(p);
        toggle!(p);
        assert!(high!(p));
    }

    #[test]
    fn level_toggle_low() {
        let p = pin!(1, "A", Unconnected);
        set!(p);
        toggle!(p);
        assert!(low!(p));
    }

    #[test]
    fn level_toggle_float() {
        let p = pin!(1, "A", Unconnected);
        float!(p);
        toggle!(p);
        assert!(floating!(p));
    }

    #[test]
    fn pull_up_initial() {
        let p = pin!(1, "A", Output);
        pull_up!(p);
        assert!(high!(p));
    }

    #[test]
    fn pull_up_unconnected() {
        let p = pin!(1, "A", Unconnected);
        pull_up!(p);

        clear!(p);
        assert!(low!(p));
        float!(p);
        assert!(high!(p));
    }

    #[test]
    fn pull_up_input() {
        let p = pin!(1, "A", Input);
        pull_up!(p);
        let t = trace!(p);

        clear!(t);
        assert!(low!(p));
        float!(t);
        assert!(high!(p));
    }

    #[test]
    fn pull_up_output() {
        let p = pin!(1, "A", Output);
        pull_up!(p);
        let t = trace!(p);

        clear!(p);
        assert!(low!(t));
        float!(p);
        assert!(high!(t));
    }

    #[test]
    fn pull_up_bidirectional() {
        let p = pin!(1, "A", Bidirectional);
        pull_up!(p);
        let t = trace!(p);

        clear!(p);
        assert!(low!(t));
        float!(p);
        assert!(high!(t));
    }

    #[test]
    fn pull_up_after() {
        let p = pin!(1, "A", Unconnected);
        assert!(floating!(p));
        pull_up!(p);
        assert!(high!(p));
    }

    #[test]
    fn pull_down_initial() {
        let p = pin!(1, "A", Output);
        pull_down!(p);
        assert!(low!(p));
    }

    #[test]
    fn pull_down_unconnected() {
        let p = pin!(1, "A", Unconnected);
        pull_down!(p);

        set!(p);
        assert!(high!(p));
        float!(p);
        assert!(low!(p));
    }

    #[test]
    fn pull_down_input() {
        let p = pin!(1, "A", Input);
        pull_down!(p);
        let t = trace!(p);

        set!(t);
        assert!(high!(p));
        float!(t);
        assert!(low!(p));
    }

    #[test]
    fn pull_down_output() {
        let p = pin!(1, "A", Output);
        pull_down!(p);
        let t = trace!(p);

        set!(p);
        assert!(high!(t));
        float!(p);
        assert!(low!(t));
    }

    #[test]
    fn pull_down_bidirectional() {
        let p = pin!(1, "A", Bidirectional);
        pull_down!(p);
        let t = trace!(p);

        set!(p);
        assert!(high!(t));
        float!(p);
        assert!(low!(t));
    }

    #[test]
    fn pull_down_after() {
        let p = pin!(1, "A", Unconnected);
        assert!(floating!(p));
        pull_down!(p);
        assert!(low!(p));
    }

    #[test]
    fn pull_off_initial() {
        let p = pin!(1, "A", Output);
        pull_off!(p);
        assert!(floating!(p));
    }

    #[test]
    fn pull_off_pull_up() {
        let p = pin!(1, "A", Output);
        pull_up!(p);

        float!(p);
        assert!(high!(p));
        pull_off!(p);
        float!(p);
        assert!(floating!(p));
    }

    #[test]
    fn pull_off_pull_down() {
        let p = pin!(1, "A", Output);
        pull_down!(p);

        float!(p);
        assert!(low!(p));
        pull_off!(p);
        float!(p);
        assert!(floating!(p));
    }

    // Device testing
    //
    // This is a bit weird because we need to be able to see into the Device from the
    // outside to check its level and/or call count. This is not the way that things will
    // work in normal operations, where the Device by design should not be looked into from
    // the outside.
    //
    // We would therefore normally define an `d` like this:
    //
    // let d: Rc<RefCell<dyn Device>> = Rc::new(RefCell::new(TestDevice::new(1)));
    //
    // But this would make `d.borrow()` a Device and not a TestDevice, so we would not have
    // access to any non-Device properties like `count` and `level`. Therefore, we just
    // define `d` in each test as an Rc<RefCell<TestDevice>>:
    //
    // let o = Rc::new(RefCell::new(TestDevice::new(1)));
    //
    // The downside is that we now have to pass this value directly to `attach`. We cannot
    // take an `Rc::clone` of it because `Rc::clone(&d)` will require an exactly-typed `d`
    // for the pass to `attach` to work (in other words, `attach!(p, d)` works, but
    // `attach!(p, Rc::clone(&d))` does not). And we cannot cast `d` after the fact, like
    // this:
    //
    // attach!(Rc::clone(&(d as Rc<RefCell<dyn Device>>)));
    //
    // because the process of casting moves ownership of `d` so that we cannot reference it
    // later in our assertions. (Creating `d` as the correct type in the first place doesn't
    // cause this move, but then as discussed above, we cannot see TestDevice properties
    // then.)
    //
    // For that reason, we create `d` as an Rc<RefCell<TestDevice>> and pass it to the
    // `attach!` macro. But that moves `d` so we can no longer use it, including ot make new
    // `Rc::clone`s. So we have to create all of the `Rc::clone`s we need *before* the
    // attach. It makes it look weird in many cases, but it works and does not have to be
    // done this way in actual code.

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
        fn update(&mut self, event: &LevelChange) {
            self.count += 1;
            self.level = level!(event.0);
        }

        fn pins(&self) -> RefVec<Pin> {
            RefVec::new()
        }

        fn registers(&self) -> Vec<u8> {
            Vec::new()
        }
    }

    #[test]
    fn observer_unconnected() {
        let p = pin!(1, "A", Unconnected);
        let t = trace!(p);

        let d = Rc::new(RefCell::new(TestDevice::new()));
        let tested = Rc::clone(&d);
        attach!(p, d);

        set!(t);
        assert_eq!(tested.borrow().count, 0);
        assert!(tested.borrow().level.is_none());
    }

    #[test]
    fn observer_input() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);

        let d = Rc::new(RefCell::new(TestDevice::new()));
        let tested = Rc::clone(&d);
        attach!(p, d);

        set!(t);
        assert_eq!(tested.borrow().count, 1);
        assert_eq!(tested.borrow().level.unwrap(), 1.0);
    }

    #[test]
    fn observer_output() {
        let p = pin!(1, "A", Output);
        let t = trace!(p);

        let d = Rc::new(RefCell::new(TestDevice::new()));
        let tested = Rc::clone(&d);
        attach!(p, d);

        set!(t);
        assert_eq!(tested.borrow().count, 0);
        assert!(tested.borrow().level.is_none());
    }

    #[test]
    fn observer_bidirectional() {
        let p = pin!(1, "A", Bidirectional);
        let t = trace!(p);

        let d = Rc::new(RefCell::new(TestDevice::new()));
        let tested = Rc::clone(&d);
        attach!(p, d);

        set!(t);
        assert_eq!(tested.borrow().count, 1);
        assert_eq!(tested.borrow().level.unwrap(), 1.0);
    }

    #[test]
    fn observer_direct() {
        let p = pin!(1, "A", Input);

        let d = Rc::new(RefCell::new(TestDevice::new()));
        let tested = Rc::clone(&d);
        attach!(p, d);

        set!(p);
        assert_eq!(tested.borrow().count, 0);
    }

    #[test]
    fn observer_detach() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);

        let d = Rc::new(RefCell::new(TestDevice::new()));
        let tested = Rc::clone(&d);

        attach!(p, d);

        set!(t);
        assert_eq!(tested.borrow().count, 1);

        detach!(p);

        clear!(t);
        assert_eq!(tested.borrow().count, 1);
    }

    #[test]
    fn observer_non_existent() {
        let p = pin!(1, "A", Input);
        let t = trace!(p);

        let d = Rc::new(RefCell::new(TestDevice::new()));
        let tested = Rc::clone(&d);

        set!(t);
        assert_eq!(tested.borrow().count, 0);

        detach!(p);

        clear!(t);
        assert_eq!(tested.borrow().count, 0);
    }
}
