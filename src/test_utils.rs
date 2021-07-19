// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use crate::{
    components::{device::DeviceRef, trace::Trace},
    vectors::RefVec,
};

pub fn make_traces(device: &DeviceRef) -> RefVec<Trace> {
    let mut v = vec![];
    for pin in device.borrow().pins().iter() {
        v.push(trace!(clone_ref!(pin)));
    }
    RefVec::with_vec(v)
}

pub fn value_to_traces(value: usize, traces: &RefVec<Trace>) {
    for (i, trace) in traces.iter_ref().enumerate() {
        set_level!(trace, Some(((value >> i) & 1) as f64));
    }
}

pub fn traces_to_value(traces: &RefVec<Trace>) -> usize {
    let mut value = 0;
    for (i, trace) in traces.iter_ref().enumerate() {
        value |= (match level!(trace) {
            Some(v) if v >= 0.5 => 1,
            _ => 0,
        }) << i;
    }
    value
}
