//! The engine requires gradual underflow. NIH-plug enables FTZ around process(),
//! so restore IEEE subnormals locally and restore the host's mode on exit.
pub struct FloatMode {
    #[cfg(target_arch = "aarch64")]
    saved: u64,
    #[cfg(target_arch = "x86_64")]
    saved: u32,
}
impl FloatMode {
    pub fn enter() -> Self {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            let saved: u64;
            core::arch::asm!("mrs {}, fpcr", out(reg) saved, options(nostack, preserves_flags));
            let ieee = saved & !(1 << 24);
            core::arch::asm!("msr fpcr, {}", in(reg) ieee, options(nostack, preserves_flags));
            Self { saved }
        }
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let mut saved = 0_u32;
            core::arch::asm!("stmxcsr [{}]", in(reg) &mut saved, options(nostack, preserves_flags));
            let ieee = saved & !((1 << 15) | (1 << 6));
            core::arch::asm!("ldmxcsr [{}]", in(reg) &ieee, options(nostack, preserves_flags));
            Self { saved }
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        Self {}
    }
}
impl Drop for FloatMode {
    fn drop(&mut self) {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("msr fpcr, {}", in(reg) self.saved, options(nostack, preserves_flags));
        }
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("ldmxcsr [{}]", in(reg) &self.saved, options(nostack, preserves_flags));
        }
    }
}
