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
