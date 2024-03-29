use std::rc::Rc;
use std::cell::Cell;

use super::{SynthModule, OutputInfo};

#[derive(Debug, Clone, Copy, Hash)]
enum Adsr {
    Attack,
    Decay,
    Sustain,
    Release,
    Done
}

#[derive(Clone)]
pub struct Envelope {
    // Times here should be in milliseconds
    attack_time: f32,
    decay_time: f32,
    sustain_level: f32,
    release_time: f32,

    stage: Cell<Adsr>,
    previous_value: Cell<f32>,

    trigger: Option<Rc<dyn SynthModule>>,
    trigger_tolerance: f32, // Minimum value at which envelope is triggered
    triggered: Cell<bool>
}

impl Envelope {
    pub fn new() -> Self {
        let attack_time = 0.0;
        let decay_time = 0.0;
        let sustain_level = 1.0;
        let release_time = 0.0;

        let stage = Cell::new(Adsr::Done);
        let previous_value = Cell::new(0.0);

        let trigger = None;
        let trigger_tolerance = 0.5;
        let triggered = Cell::new(false);

        Self { 
            attack_time, decay_time, sustain_level, release_time,
            stage, previous_value, trigger, trigger_tolerance, triggered
        }
    }

    pub fn set_attack_time(&mut self, attack_time: f32) {
        self.attack_time = attack_time;
    }

    pub fn get_attack_time(&self) -> f32 {
        self.attack_time
    }
    
    pub fn set_decay_time(&mut self, decay_time: f32) {
        self.decay_time = decay_time;
    }

    pub fn get_decay_time(&self) -> f32 {
        self.decay_time
    }

    pub fn set_sustain_level(&mut self, sustain_level: f32) {
        self.sustain_level = sustain_level;
    }

    pub fn get_sustain_level(&self) -> f32 {
        self.sustain_level
    }

    pub fn set_release_time(&mut self, release_time: f32) {
        self.release_time = release_time;
    }

    pub fn get_release_time(&self) -> f32 {
        self.release_time
    }

    pub fn set_trigger(&mut self, trigger: Option<Rc<dyn SynthModule>>) {
        self.trigger = trigger;
    }

    pub fn set_trigger_tolerance(&mut self, trigger_tolerance: f32) {
        self.trigger_tolerance = trigger_tolerance;
    }

    pub fn get_trigger_tolerance(&self) -> f32 {
        self.trigger_tolerance
    }

    pub fn trigger(&self) {
        self.stage.set(Adsr::Attack);
        self.triggered.set(true);
    }

    pub fn release(&self) {
        self.stage.set(Adsr::Release);
        self.triggered.set(false);
    }

    pub fn copy_state_from(&mut self, other: &Self) {
        // Note: Does not update trigger connection
        self.attack_time = other.attack_time;
        self.decay_time = other.decay_time;
        self.sustain_level = other.sustain_level;
        self.release_time = other.release_time;
        self.trigger_tolerance = other.trigger_tolerance;
    }

    fn get_attack(&self, sample_rate: usize) -> f32 {
        let time_in_milliseconds = 1000.0 / sample_rate as f32;
        let increase_factor = time_in_milliseconds / self.attack_time;
        let envelope_value = self.previous_value.get() + increase_factor;
        debug_assert!(envelope_value.is_finite());
        if envelope_value >= 1.0 {
            if self.decay_time > 0.0 && self.sustain_level != 1.0 {
                // There is a decay stage
                self.stage.set(Adsr::Decay);
                self.previous_value.set(1.0);
                return 1.0;
            }
            else {
                // There is no decay stage
                self.stage.set(Adsr::Sustain);
                self.previous_value.set(self.sustain_level);
                return 1.0
            }
            
        }
        self.previous_value.set(envelope_value);
        envelope_value
    }

    fn get_decay(&self, sample_rate: usize) -> f32 {
        let time_in_milliseconds = 1000.0 / sample_rate as f32;
        let decrease_factor = time_in_milliseconds * (1.0 - self.sustain_level) / self.decay_time;
        let envelope_value = self.previous_value.get() - decrease_factor;
        debug_assert!(envelope_value.is_finite());
        if envelope_value <= self.sustain_level {
            self.stage.set(Adsr::Sustain);
            self.previous_value.set(self.sustain_level);
            return self.sustain_level;
        }
        self.previous_value.set(envelope_value);
        envelope_value
    }

    fn get_release(&self, sample_rate: usize) -> f32 {
        let time_in_milliseconds = 1000.0 / sample_rate as f32;
        let decrease_factor = time_in_milliseconds / self.release_time;
        let envelope_value = self.previous_value.get() - decrease_factor;
        debug_assert!(envelope_value.is_finite());
        if envelope_value <= 0.0 {
            self.stage.set(Adsr::Done);
            self.previous_value.set(0.0);
            return 0.0;
        }
        self.previous_value.set(envelope_value);
        envelope_value
    }

    pub fn get(&self, sample_rate: usize) -> f32 {
        match self.stage.get() {
            Adsr::Attack  => self.get_attack(sample_rate),
            Adsr::Decay   => self.get_decay(sample_rate),
            Adsr::Sustain => self.sustain_level,
            Adsr::Release => self.get_release(sample_rate),
            Adsr::Done    => 0.0
        }
    }
}

impl SynthModule for Envelope {
    fn fill_output_buffer(&self, data: &mut [f32], output_info: &OutputInfo) {
        let data_size = data.len();
        let mut trigger_data = Vec::with_capacity(data_size);
        trigger_data.resize(data_size, 0.0);

        if let Some(trigger) = &self.trigger {
            trigger.fill_output_buffer(&mut trigger_data, output_info);
        }
        else {
            data.fill(0.0);
            return;
        }

        for (i, datum) in data.iter_mut().enumerate() {
            let triggered = trigger_data[i] > self.trigger_tolerance;
            if triggered != self.triggered.get() {
                // Triggered state has changed. We should either start attack or release
                if triggered {
                    self.trigger();
                }
                else {
                    self.release();
                }
            }
            *datum = self.get(output_info.sample_rate);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::clock;

    struct ConstantTrigger;
    impl SynthModule for ConstantTrigger {
        fn fill_output_buffer(&self, data: &mut [f32], _output_info: &OutputInfo) {
            for datum in data.iter_mut() {
                *datum = 1.0;
            }
        }
    }

    struct SplitTrigger;
    impl SynthModule for SplitTrigger {
        fn fill_output_buffer(&self, data: &mut [f32], _output_info: &OutputInfo) {
            let (trigger_data, untrigger_data) = data.split_at_mut(data.len() / 2);
            for datum in trigger_data.iter_mut() {
                *datum = 1.0;
            }
            for datum in untrigger_data.iter_mut() {
                *datum = 0.0;
            }
        }
    }

    fn create_output_info(sample_rate: usize, buffer_size: usize) -> OutputInfo {
        let mut clock = clock::SampleClock::new(sample_rate);
        let clock_values = clock.get_range(buffer_size);
        OutputInfo::new_basic(sample_rate, clock_values)
    }

    #[test]
    fn test_basic_envelope_with_sustain() {
        const SAMPLE_RATE: usize = 4_usize;
        const EXPECTED_DATA: [f32; 12] = [0.25, 0.5, 0.75, 1.0, 0.9375, 0.875, 0.8125, 0.75, 0.75, 0.75, 0.75, 0.75];

        let mut envelope = Envelope::new();
        envelope.set_attack_time(1000.0);
        envelope.set_decay_time(1000.0);
        envelope.set_sustain_level(0.75);
        envelope.set_release_time(1000.0);

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        let trigger = ConstantTrigger {};
        envelope.set_trigger(Some(Rc::new(trigger)));

        let mut data = Vec::with_capacity(SAMPLE_RATE * 3);
        data.resize(SAMPLE_RATE * 3, 0.0);
        envelope.fill_output_buffer(&mut data, &output_info);

        for (got_datum, expected_datum) in data.iter().zip(EXPECTED_DATA.iter()) {
            assert!(
                float_eq(*got_datum, *expected_datum, 0.0001),
                "Envelope output does not match expected:\n\tGot: {:?}\n\tExpected: {:?}", data, EXPECTED_DATA
            );
        }
    }

    #[test]
    fn test_basic_envelope_with_release() {
        const SAMPLE_RATE: usize = 4_usize;
        const EXPECTED_DATA: [f32; 16] = [0.25, 0.5, 0.75, 1.0, 0.875, 0.75, 0.625, 0.5, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

        let mut envelope = Envelope::new();
        envelope.set_attack_time(1000.0);
        envelope.set_decay_time(1000.0);
        envelope.set_sustain_level(0.5);
        envelope.set_release_time(1000.0);

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        let trigger = SplitTrigger {};
        envelope.set_trigger(Some(Rc::new(trigger)));

        let mut data = Vec::with_capacity(SAMPLE_RATE * 4);
        data.resize(SAMPLE_RATE * 4, 0.0);
        envelope.fill_output_buffer(&mut data, &output_info);

        for (got_datum, expected_datum) in data.iter().zip(EXPECTED_DATA.iter()) {
            assert!(
                float_eq(*got_datum, *expected_datum, 0.0001),
                "Envelope output does not match expected:\n\tGot: {:?}\n\tExpected: {:?}", data, EXPECTED_DATA
            );
        }
    }
}