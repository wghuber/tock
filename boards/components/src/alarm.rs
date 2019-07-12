//! Component for harware timer Alarms on the imix board.
//!
//! This provides one component, AlarmDriverComponent, which provides
//! an alarm system call interface.
//!
//! Usage
//! -----
//! ```rust
//! let alarm = AlarmDriverComponent::new(mux_alarm).finalize();
//! ```

// Author: Philip Levis <pal@cs.stanford.edu>
// Last modified: 6/20/2018

#![allow(dead_code)] // Components are intended to be conditionally included

use capsules::alarm::AlarmDriver;
use capsules::virtual_alarm::{MuxAlarm, VirtualMuxAlarm};
use kernel::capabilities;
use kernel::component::Component2;
use kernel::create_capability;
use kernel::hil::time;

// Setup static space for the objects.
#[macro_export]
macro_rules! alarm_component_helper {
    ($A:ty) => {
        {
            use capsules::alarm::AlarmDriver;
            static mut BUF1: Option<VirtualMuxAlarm<'static, $A>> = None;
            static mut BUF2: Option<AlarmDriver<'static, VirtualMuxAlarm<'static, $A>>> = None;
            (&mut BUF1, &mut BUF2)
        };
    }
}

pub struct AlarmDriverComponent<A: 'static + time::Alarm> {
    board_kernel: &'static kernel::Kernel,
    alarm_mux: &'static MuxAlarm<'static, A>,
}

impl<A: 'static + time::Alarm> AlarmDriverComponent<A> {
    pub fn new(
        board_kernel: &'static kernel::Kernel,
        mux: &'static MuxAlarm<'static, A>,
    ) -> AlarmDriverComponent<A> {
        AlarmDriverComponent {
            board_kernel: board_kernel,
            alarm_mux: mux,
        }
    }
}

impl<A: 'static + time::Alarm> Component2 for AlarmDriverComponent<A> {
    type InputBuffer = (&'static mut Option<VirtualMuxAlarm<'static, A>>,
              &'static mut Option<AlarmDriver<'static, VirtualMuxAlarm<'static, A>>>
              );
    type Output = &'static AlarmDriver<'static, VirtualMuxAlarm<'static, A>>;

    unsafe fn finalize(&mut self, static_buffer: Self::InputBuffer) -> Self::Output {
        let grant_cap = create_capability!(capabilities::MemoryAllocationCapability);

        let virtual_alarm1 = static_init_h!(
            static_buffer.0,
            VirtualMuxAlarm<'static, A>,
            VirtualMuxAlarm::new(self.alarm_mux)
        );
        let alarm = static_init_h!(
            static_buffer.1,
            AlarmDriver<'static, VirtualMuxAlarm<'static, A>>,
            AlarmDriver::new(virtual_alarm1, self.board_kernel.create_grant(&grant_cap))
        );

        virtual_alarm1.set_client(alarm);
        alarm
    }
}
