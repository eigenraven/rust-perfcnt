use std::fs::File;
use std::os::unix::io::FromRawFd;
use std::io::Read;
use std::io;
use std::io::{Error};
use std::mem;

use libc::{pid_t};

#[allow(dead_code, non_camel_case_types)]
mod hw_breakpoint;
#[allow(dead_code, non_camel_case_types)]
mod perf_event;

use ::AbstractPerfCounter;
use x86::perfcnt::intel::description::{IntelPerformanceCounterDescription, Tuple};

const IOCTL: usize = 16;
const PERF_EVENT_OPEN: usize = 298;

fn perf_event_open(hw_event: perf_event::perf_event_attr,
                       pid: perf_event::__kernel_pid_t,
                       cpu:  ::libc::c_int,
                       group_fd:  ::libc::c_int,
                       flags:  ::libc::c_int) -> isize {
    unsafe {
        syscall!(PERF_EVENT_OPEN, &hw_event as *const perf_event::perf_event_attr as usize, pid, cpu, group_fd, flags) as isize
    }
}

fn ioctl(fd: ::libc::c_int, request: u64, value: ::libc::c_int) -> isize {
    unsafe {
        syscall!(IOCTL, fd, request, value) as isize
    }
}

fn read_counter(fd: ::libc::c_int) -> Result<u64, io::Error> {
    unsafe {
        let mut f = File::from_raw_fd(fd);
        let mut value: [u8; 8] = Default::default();
        try!(f.read(&mut value));
        Ok(mem::transmute::<[u8; 8], u64>(value))
    }
}

pub struct PerfCounterBuilder {
    group: isize,
    pid: pid_t,
    cpu: isize,
    flags: usize,
    attrs: perf_event::perf_event_attr,
}

impl Default for PerfCounterBuilder {
    fn default () -> PerfCounterBuilder {
        PerfCounterBuilder {
            group: -1,
            pid: 0,
            cpu: -1,
            flags: 0,
            attrs: Default::default()
        }
    }
}

impl PerfCounterBuilder {

    pub fn new() -> PerfCounterBuilder {
        Default::default()
    }

    pub fn set_group<'a>(&'a mut self, group_fd: isize) -> &'a mut PerfCounterBuilder {
        self.group = group_fd;
        self
    }

    /// Sets PERF_FLAG_FD_OUTPUT
    ///
    /// This flag re-routes the output from an event to the group leader.
    pub fn set_flag_fd_output<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.flags |= 0x0; //PERF_FLAG_FD_OUTPUT;
        self
    }

    /// Sets PERF_FLAG_PID_CGROUP
    ///
    /// This flag activates per-container system-wide monitoring.  A
    /// container is an abstraction that isolates a set of resources for
    /// finer grain control (CPUs, memory, etc.).   In  this  mode,  the
    /// event  is  measured  only if the thread running on the monitored
    /// CPU belongs to the designated container (cgroup).
    pub fn set_flag_pid_cgroup<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.flags |= 0x0; //PERF_FLAG_PID_CGROUP;
        self
    }

    pub fn set_sample_period<'a>(&'a mut self, period: u64) -> &'a mut PerfCounterBuilder {
        self.attrs.sample_period_freq = period;
        self
    }

    pub fn set_sample_frequency<'a>(&'a mut self, frequency: u64) -> &'a mut PerfCounterBuilder {
        self.attrs.sample_period_freq = frequency;
        self.attrs.settings.insert(perf_event::EVENT_ATTR_FREQ);
        self
    }

    /// The counter starts out disabled.
    pub fn disable<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_DISABLED);
        self
    }

    /// This counter should count events of child tasks as well as the task specified.
    pub fn inherit<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_INHERIT);
        self
    }

    /// The pinned bit specifies that the counter should always be on the CPU if at all possible.
    /// It applies only to  hardware counters and only to group leaders.
    pub fn pinned<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_PINNED);
        self
    }

    /// The counter is exclusive i.e., when this counter's group is on the CPU,
    /// it should be the only group using the CPU's counters.
    pub fn exclusive<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_EXCLUSIVE);
        self
    }

    /// The counter excludes events that happen in user space.
    pub fn exclude_user<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_EXCLUDE_USER);
        self
    }

    /// The counter excludes events that happen in the kernel.
    pub fn exclude_kernel<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_EXCLUDE_KERNEL);
        self
    }

    /// The counter excludes events that happen in the hypervisor.
    pub fn exclude_hv<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_EXCLUDE_HV);
        self
    }

    /// The counter doesn't count when the CPU is idle.
    pub fn exclude_idle<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_EXCLUDE_IDLE);
        self
    }

    /// Enables recording of exec mmap events.
    pub fn enable_mmap<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_MMAP);
        self
    }

    /// The counter will save event counts on context switch for inherited tasks.
    /// This is meaningful only if the inherit field is set.
    pub fn inherit_stat<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_INHERIT_STAT);
        self
    }

    /// The counter is automatically enabled after a call to exec.
    pub fn enable_on_exec<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_ENABLE_ON_EXEC);
        self
    }

    /// fork/exit notifications are included in the ring buffer.
    pub fn enable_task_notification<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_TASK);
        self
    }

    /// The counter has  a  sampling  interrupt happen when we cross the wakeup_watermark
    /// boundary.  Otherwise interrupts happen after wakeup_events samples.
    pub fn enable_watermark<'a>(&'a mut self, watermark_events: u32) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_WATERMARK);
        self.attrs.wakeup_events_watermark = watermark_events;
        self
    }

    /// Sampled IP counter can have arbitrary skid.
    pub fn set_ip_sample_arbitrary_skid<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_SAMPLE_IP_ARBITRARY_SKID);
        self
    }

    /// Sampled IP counter requested to have constant skid.
    pub fn set_ip_sample_constant_skid<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_SAMPLE_IP_CONSTANT_SKID);
        self
    }

    /// Sampled IP counter requested to have 0 skid.
    pub fn set_ip_sample_req_zero_skid<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_SAMPLE_IP_REQ_ZERO_SKID);
        self
    }

    /// The counterpart of enable_mmap, but enables including data mmap events in the ring-buffer.
    pub fn enable_mmap_data<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_MMAP_DATA);
        self
    }

    /// Sampled IP counter must have 0 skid.
    pub fn set_ip_sample_zero_skid<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.attrs.settings.insert(perf_event::EVENT_ATTR_SAMPLE_IP_ZERO_SKID);
        self
    }

    pub fn from_raw_intel_hw_counter<'a>(&'a mut self, counter: &IntelPerformanceCounterDescription) -> &'a mut PerfCounterBuilder {
        let mut config: u64 = 0;

        match counter.event_code {
            Tuple::One(code) =>  config |= (code as u64) << 0,
            Tuple::Two(_, _) => unreachable!() // NYI
        };
        match counter.umask {
            Tuple::One(code) =>  config |= (code as u64) << 8,
            Tuple::Two(_, _) => unreachable!() // NYI
        };
        config |= (counter.counter_mask as u64) << 24;

        if counter.edge_detect {
            config |= 1 << 18;
        }
        if counter.any_thread {
            config |= 1 << 21;
        }
        if counter.invert {
            config |= 1 << 23;
        }

        self.attrs._type = perf_event::PERF_TYPE_RAW;
        self.attrs.config = config;
        self
    }

    pub fn for_all_pids<'a>(&'a mut self) ->  &'a mut PerfCounterBuilder {
        self.pid = -1;
        self
    }

    pub fn for_pid<'a>(&'a mut self, pid: i32) -> &'a mut PerfCounterBuilder {
        self.pid = pid;
        self
    }

    pub fn on_cpu<'a>(&'a mut self, cpu: isize) -> &'a mut PerfCounterBuilder {
        self.cpu = cpu;
        self
    }

    pub fn on_all_cpus<'a>(&'a mut self) -> &'a mut PerfCounterBuilder {
        self.cpu = -1;
        self
    }

    pub fn finish(&self) -> PerfCounter {
        let flags = 0;
        let fd = perf_event_open(self.attrs, self.pid, self.cpu as i32, self.group as i32, flags) as ::libc::c_int;
        if fd < 0 {
            println!("Error opening leader {:?}", fd);
        }

        PerfCounter { fd: fd }
    }
}

pub struct PerfCounter {
    fd: ::libc::c_int
}

impl AbstractPerfCounter for PerfCounter {

    fn reset(&self) -> Result<(), io::Error> {
        let ret = ioctl(self.fd, perf_event::PERF_EVENT_IOC_RESET, 0);
        if ret == -1 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }

    fn start(&self) -> Result<(), io::Error> {
        let ret = ioctl(self.fd, perf_event::PERF_EVENT_IOC_ENABLE, 0);
        if ret == -1 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }

    fn stop(&self) -> Result<(), io::Error> {
        let ret = ioctl(self.fd, perf_event::PERF_EVENT_IOC_DISABLE, 0);
        if ret == -1 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }

    fn read(&self) -> Result<u64, io::Error> {
        read_counter(self.fd)
    }
}
