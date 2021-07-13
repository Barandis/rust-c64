// Copyright (c) 2021 Thomas J. Otterson
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use crate::components::{device::DeviceRef, trace::TraceRef};

pub fn make_traces(device: DeviceRef) -> Vec<TraceRef> {
    let mut v = vec![];
    for pin in device.borrow().pins().iter() {
        v.push(trace!(clone_ref!(pin)));
    }
    v
}

pub fn value_to_traces(value: usize, traces: Vec<TraceRef>) {
    for (i, trace) in traces.into_iter().enumerate() {
        set_level!(trace, Some(((value >> i) & 1) as f64));
    }
}

pub fn traces_to_value(traces: Vec<TraceRef>) -> usize {
    let mut value = 0;
    for (i, trace) in traces.into_iter().enumerate() {
        value |= (match level!(trace) {
            Some(v) if v >= 0.5 => 1,
            _ => 0,
        }) << i;
    }
    value
}
