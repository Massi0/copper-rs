use bincode::{Decode, Encode};

use cu29::clock::RobotClock;
use cu29::config::ComponentConfig;
use cu29::cutask::{CuMsg, CuSinkTask, CuSrcTask, CuTask, CuTaskLifecycle, Freezable};
use cu29::CuResult;
use cu29::{input_msg, output_msg};

use cu29_log_derive::debug;

// Define a message type
#[derive(Default, Debug, Clone, Encode, Decode)]
pub struct IntRange {
    a: u32,
    b: u32,
}

#[derive(Default, Debug, Clone, Encode, Decode)]
pub struct IntList {
    value: Vec<u32>,
}

// Defines a source (ie. driver)
#[derive(Default)]
pub struct Source {}

impl IntRange {
    // Method to unwrap the struct into two values
    pub fn unwrap(self) -> (u32, u32) {
        (self.a, self.b)
    }
}

// Needs to be fully implemented if you want to have a stateful task.
impl Freezable for Source {}

impl CuTaskLifecycle for Source {
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
    }
    // don't forget the other lifecycle methods if you need them: start, stop, preprocess, postprocess
}

impl<'cl> CuSrcTask<'cl> for Source {
    type Output = output_msg!('cl, IntRange);

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        // Generated a 42 message.
        output.set_payload(IntRange { a: 42, b: 150 });
        Ok(())
    }
}

// Defines a processing task
pub struct GeneratePrime {
    // if you add some task state here, you need to implement the Freezable trait
}

// Needs to be fully implemented if you want to have a stateful task.
impl Freezable for GeneratePrime {}

impl CuTaskLifecycle for GeneratePrime {
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where
        Self: Sized,
    {
        // add the task state initialization here
        Ok(Self {})
    }

    // don't forget the other lifecycle methods if you need them: start, stop, preprocess, postprocess
}

fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    for i in 2..=((n as f64).sqrt() as u32) {
        if n % i == 0 {
            return false;
        }
    }
    true
}

fn primes_between(start: u32, end: u32) -> Vec<u32> {
    (start..=end).filter(|&x| is_prime(x)).collect()
}

impl<'cl> CuTask<'cl> for GeneratePrime {
    type Input = input_msg!('cl, IntRange);
    type Output = output_msg!('cl, IntList);

    fn process(
        &mut self,
        _clock: &RobotClock,
        input: Self::Input,
        output: Self::Output,
    ) -> CuResult<()> {
        let (start, end) = input.payload().unwrap().clone().unwrap();
        debug!("Received message: [{}-{}]", start, end);
        output.set_payload(IntList {
            value: primes_between(start, end),
        });
        Ok(()) // outputs another message for downstream
    }
}

// Defines a sink (ie. actualtion)
#[derive(Default)]
pub struct Sink {}

// Needs to be fully implemented if you want to have a stateful task.
impl Freezable for Sink {}

impl CuTaskLifecycle for Sink {
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self>
    where
        Self: Sized,
    {
        Ok(Self {})
    }
    // don't forget the other lifecycle methods if you need them: start, stop, preprocess, postprocess
}

impl<'cl> CuSinkTask<'cl> for Sink {
    type Input = input_msg!('cl, IntList);

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        let prime_numbers = &input.payload().unwrap().value;

        let numbers_str = prime_numbers
            .iter() // Iterate over the vector
            .map(|n| n.to_string()) // Convert each element to a string
            .collect::<Vec<String>>() // Collect into a Vec<String>
            .join(", ");
        debug!(
            "Prime numbers size:{}, values: {}",
            prime_numbers.len(),
            numbers_str
        );
        Ok(())
    }
}
