//  PROFILING.rs
//    by Lut99
// 
//  Created:
//    15 Jan 2023, 16:28:37
//  Last edited:
//    15 Jan 2023, 18:14:31
//  Auto updated?
//    Yes
// 
//  Description:
//!   Defines useful structs that we can use during profiling performance
//!   of the framework (and also workflows / tasks, while at it).
// 

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result as FResult};
use std::io::Write;
use std::time::{Duration, Instant};

use log::error;
use serde::{Deserialize, Serialize};


/***** HELPER FUNCTIONS *****/
/// Writes the TimingReport to the given writer.
/// 
/// # Arguments
/// - `writer`: The `Write`r to write to.
/// - `name`: The name of the timing report to write.
/// - `timings`: The timings in the timing report to write.
/// 
/// # Errors
/// This function errors if we failed to write to the given writer.
fn write_report(mut writer: impl Write, name: impl AsRef<str>, timings: &HashMap<String, RefCell<Timing>>) -> Result<(), std::io::Error> {
    writeln!(&mut writer, "Timing report '{}':", name.as_ref())?;
    for (what, timing) in timings {
        writeln!(&mut writer, "  - Timing results for {}: {}", what, timing.borrow().display())?;
    }

    // Done
    Ok(())
}





/***** AUXILLARY *****/
/// Formats the giving Timing to show a (hopefully) sensible scale to the given formatter.
#[derive(Debug)]
pub struct TimingFormatter<'t>(&'t Timing);
impl<'t> Display for TimingFormatter<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        if self.0.nanos < 1_000 { write!(f, "{}ns", self.0.nanos) }
        else if self.0.nanos < 1_000_000 { write!(f, "{}us", self.0.nanos / 1_000) }
        else if self.0.nanos < 1_000_000_000 { write!(f, "{}ms", self.0.nanos / 1_000_000) }
        else { write!(f, "{}s", self.0.nanos / 1_000_000_000) }
    }
}



/// Defines the TimingGuard, which takes timings until it goes out-of-scope and shows them to stdout.
#[derive(Clone, Debug)]
pub struct TimingGuard<'t> {
    /// The start time of the guard.
    start  : Instant,
    /// The timing that we want to populate, eventually.
    timing : &'t RefCell<Timing>,
}
impl<'t> TimingGuard<'t> {
    /// Consumes this TimingGuard to return the time early.
    #[inline]
    pub fn stop(self) {}
}
impl<'t> Drop for TimingGuard<'t> {
    fn drop(&mut self) {
        // Update the time it took us in the internal timing
        *self.timing.borrow_mut() = self.start.elapsed().into();
    }
}





/***** LIBRARY *****/
/// Defines a taken Timing, which represents an amount of time that has passed.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Timing {
    /// The amount of nanoseconds that have passed.
    nanos : u128,
}

impl Timing {
    /// Returns a Timing in which no time has passed.
    /// 
    /// # Returns
    /// A new Timing instance, for which all `Timing::elapsed_XX()` functions will return 0.
    #[inline]
    pub const fn none() -> Self {
        Self{ nanos : 0 }
    }



    /// Writes a human-readable representation of the elapsed time in this Timing.
    /// 
    /// Will attempt to find the correct scale automagically; specifically, will try to write as seconds _unless_ the time is less than that. Then, it will move to milliseconds, all the way up to nanoseconds.
    /// 
    /// # Returns
    /// A TimingFormatter that implements Display to do this kind of formatting on this Timing.
    #[inline]
    pub fn display(&self) -> TimingFormatter { TimingFormatter(self) }

    /// Returns the time that has been elapsed, in seconds.
    /// 
    /// # Returns
    /// The elapsed time that this Timing represents in seconds.
    #[inline]
    pub const fn elapsed_s(&self) -> u128 { self.nanos / 1_000_000_000 }

    /// Returns the time that has been elapsed, in milliseconds.
    /// 
    /// # Returns
    /// The elapsed time that this Timing represents in milliseconds.
    #[inline]
    pub const fn elapsed_ms(&self) -> u128 { self.nanos / 1_000_000 }

    /// Returns the time that has been elapsed, in microseconds.
    /// 
    /// # Returns
    /// The elapsed time that this Timing represents in microseconds.
    #[inline]
    pub const fn elapsed_us(&self) -> u128 { self.nanos / 1_000 }

    /// Returns the time that has been elapsed, in nanoseconds.
    /// 
    /// # Returns
    /// The elapsed time that this Timing represents in nanoseconds.
    #[inline]
    pub const fn elapsed_ns(&self) -> u128 { self.nanos }
}

impl AsRef<Timing> for Timing {
    #[inline]
    fn as_ref(&self) -> &Self { self }
}
impl From<&Timing> for Timing {
    #[inline]
    fn from(value: &Timing) -> Self { *value }
}
impl From<&mut Timing> for Timing {
    #[inline]
    fn from(value: &mut Timing) -> Self { *value }
}

impl From<Duration> for Timing {
    #[inline]
    fn from(value: Duration) -> Self { Timing{ nanos: value.as_nanos() } }
}
impl From<&Duration> for Timing {
    #[inline]
    fn from(value: &Duration) -> Self { Timing{ nanos: value.as_nanos() } }
}
impl From<&mut Duration> for Timing {
    #[inline]
    fn from(value: &mut Duration) -> Self { Timing{ nanos: value.as_nanos() } }
}



/// Defines the TimingReport, which can be used to output various profile things to some file.
#[derive(Deserialize, Serialize)]
pub struct TimingReport<'w> {
    /// If not None, points to something to report the findings to once the TimingReport goes out-of-scope.
    #[serde(skip)]
    auto_report : Option<Box<dyn 'w + Write>>,

    /// The name of the report.
    name    : String,
    /// The timings hidden within this report, together with their description.
    timings : HashMap<String, RefCell<Timing>>,
}

impl<'w> TimingReport<'w> {
    /// Constructor for the TimingReport that initializes it to new.
    /// 
    /// # Arguments
    /// - `name`: The name of the report so that it is identifyable in case multiple reports are being written.
    /// 
    /// # Returns
    /// A new TimingReport that can be used to report... timings...
    #[inline]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            auto_report : None,

            name    : name.into(),
            timings : HashMap::new(),
        }
    }

    /// Constructor for the TimingReport that will automatically report it to the given `Write`r when it goes out-of-scope.
    /// 
    /// # Arguments
    /// - `name`: The name of the report so that it is identifyable in case multiple reports are being written.
    /// - `writer`: The `Write`r to write to once we drop ourselves. Note that any failure to write to this writer will be reported using `error!()`, but not considered fatal.
    /// 
    /// # Returns
    /// A new TimingReport that can be used to report... timings...
    #[inline]
    pub fn auto_report(name: impl Into<String>,writer: impl 'w + Write) -> Self {
        Self {
            auto_report : Some(Box::new(writer)),

            name    : name.into(),
            timings : HashMap::new(),
        }
    }



    /// Record a given Timing in this report.
    /// 
    /// If the given `what` already exists, overrides the timing for it silently.
    /// 
    /// # Arguments
    /// - `what`: Some description of the Timing we are reporting. Should fill in the blank in `Timing results for ...`.
    /// - `timing`: The Timing to register.
    #[inline]
    pub fn add(&mut self, what: impl Into<String>, timing: impl Into<Timing>) {
        if self.timings.capacity() == self.timings.len() { self.timings.reserve(self.timings.len()); }
        self.timings.insert(what.into(), RefCell::new(timing.into()));
    }

    /// Start recording a timing in this report, returning a TimingGuard that can be used to stop it.
    /// 
    /// The timing will automatically stop if the TimingGuard goes out-of-scope.
    /// 
    /// # Arguments
    /// - `what`: Some description of the Timing we are reporting. Should fill in the blank in `Timing results for ...`.
    /// 
    /// # Returns
    /// A TimingGuard instance that can be used to finalize the timing.
    #[inline]
    pub fn guard(&mut self, what: impl Into<String>) -> TimingGuard {
        // Return that timing to be populated by the TimingGuard.
        if self.timings.capacity() == self.timings.len() { self.timings.reserve(self.timings.len()); }
        TimingGuard {
            start  : Instant::now(),
            timing : self.timings.entry(what.into()).or_insert(RefCell::new(Timing::none()))
        }
    }

    /// Adds the time the given function takes to the report.
    /// 
    /// # Arguments
    /// - `what`: Some description of the Timing we are reporting. Should fill in the blank in `Timing results for ...`.
    /// - `func`: The Function to profile.
    /// 
    /// # Returns
    /// The result of the function we've profiled.
    #[inline]
    pub fn func<R>(&mut self, what: impl Into<String>, func: impl FnOnce() -> R) -> R {
        // Profile the function
        let start   : Instant = Instant::now();
        let res     : R       = func();
        let elapsed : Timing  = start.elapsed().into();

        // Insert the timing and return the result
        self.timings.insert(what.into(), RefCell::new(elapsed));
        res
    }
}
impl<'w> Drop for TimingReport<'w> {
    fn drop(&mut self) {
        // If we automatically report, do so
        if let Some(writer) = &mut self.auto_report {
            if let Err(err) = write_report(writer, &self.name, &self.timings) { error!("Failed to automatically report TimingReport '{}': {}", self.name, err); }
        }
    }
}

impl<'w> Debug for TimingReport<'w> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FResult {
        // Unpack the report for better knowing when we have to extend upon this function
        let Self{ auto_report, name, timings } = self;

        // Print it
        if f.alternate() {
            writeln!(f, "TimingReport{{")?;
            writeln!(f, "    auto_report : {}", if auto_report.is_some() { "Some(Box<dyn Write>)" } else { "None" })?;
            writeln!(f, "    name        : {}", name)?;
            writeln!(f, "    timings     : {:#?}", timings)?;
            writeln!(f, "}}")?;
        } else {
            write!(f, "TimingReport{{ auto_report: {}, name: {}, timings: {:?} }}", if auto_report.is_some() { "Some(Box<dyn Write>)" } else { "None" }, name, timings)?;
        }

        // Done
        Ok(())
    }
}
