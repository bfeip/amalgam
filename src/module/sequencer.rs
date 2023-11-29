use std::rc::Rc;
use std::cell::Cell;

use super::{SynthModule, OutputInfo, EdgeDetection};
use crate::{SynthError, SynthResult};

const DEFAULT_STEP_INFO: StepInfo = StepInfo {
    kind: SequencerStepKind::Normal,
    value: 0.0_f32,
    slide: 0.0_f32
};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SequencerStepKind {
    Normal,
    Skip,
    Repeat
}

#[derive(Copy, Clone)]
pub struct StepInfo {
    pub kind: SequencerStepKind,
    pub value: f32,
    pub slide: f32
}

// TODO: Sequence direction e.g. forward, backward, forward/backward
pub struct Sequencer {
    steps: Vec<StepInfo>,
    playing: Cell<bool>,
    cycle: bool,
    current_step: Cell<usize>,

    clock: Option<Rc<dyn SynthModule>>,
    edge_detection: EdgeDetection,
    edge_tolerance: f32
}

impl Sequencer {
    pub fn new() -> Self {
        let steps = Vec::new();
        let playing = Cell::new(false);
        let cycle = true;
        let current_step = Cell::new(0_usize);

        let clock = None;
        let edge_detection = EdgeDetection::Falling;
        let edge_tolerance = 0.8_f32;
        Self { steps, playing, cycle, current_step, clock, edge_detection, edge_tolerance }
    }

    pub fn with_steps(step_count: usize) -> Self {
        let mut steps = Vec::with_capacity(step_count);
        for _ in 0..step_count {
            steps.push(DEFAULT_STEP_INFO);
        }

        let playing = Cell::new(false);
        let cycle = true;
        let current_step = Cell::new(0_usize);

        let clock = None;
        let edge_detection = EdgeDetection::Falling;
        let edge_tolerance = 0.8_f32;
        Self { steps, playing, cycle, current_step, clock, edge_detection, edge_tolerance }
    }

    pub fn add_step(&mut self) {
        self.steps.push(DEFAULT_STEP_INFO);
    }

    pub fn add_step_with_info(&mut self, info: &StepInfo) {
        self.steps.push(*info);
    }

    pub fn get_step_info(&self, step_index: usize) -> Option<&StepInfo> {
        self.steps.get(step_index)
    }

    pub fn get_step_info_mut(&mut self, step_index: usize) -> Option<&mut StepInfo> {
        self.steps.get_mut(step_index)
    }

    pub fn get_current_step_info(&self) -> Option<&StepInfo> {
        self.steps.get(self.current_step.get())
    }

    pub fn set_step_info(&mut self, step_index: usize, step_info: &StepInfo) -> SynthResult<()> {
        match self.steps.get_mut(step_index) {
            Some(step) => *step = *step_info,
            None => {
                let msg = "Failed to set sequencer step info because index was out of bounds";
                return Err(SynthError::new(msg));
            }
        }
        Ok(())
    }

    pub fn remove_step(&mut self, step_index: usize) -> SynthResult<()> {
        if step_index > self.steps.len() {
            let msg = "Failed to remove sequencer step because index is out of bounds";
            return Err(SynthError::new(msg));
        }

        self.steps.remove(step_index);
        Ok(())
    }

    pub fn increment_step(&self, force: bool) {
        self.increment_step_body(force, true);
    }

    // This is the recursive component of increment_step. it has the additional
    // parameter of needs_skip_check to note weather we need to check for the 
    // case where all steps are skip so we don't have to iterate every step
    // every time there's a skip step.
    fn increment_step_body(&self, force: bool, needs_skip_check: bool) {
        let sequence_length = self.steps.len();
        if sequence_length == 0 {
            // There are no steps, bail
            return;
        }
        if !force && !self.cycle && self.current_step.get() == sequence_length - 1 {
            // We're on the last step and we're not cycling and it's not being forced, do nothing
            return;
        }

        // Set us to the next step
        if self.steps[self.current_step.get()].kind == SequencerStepKind::Repeat {
            self.current_step.set(0);
        } else {
            self.current_step.set(self.current_step.get() + 1);
            if self.current_step.get() % sequence_length == 0 {
                if !self.cycle {
                    self.playing.set(false);
                    self.current_step.set(self.current_step.get() - 1);
                }
                else {
                    self.current_step.set(0); // return to 0 even if not cycle
                }
            }
        }

        // Check if the step we're on now is a skipped step. If it is, recurse
        if self.steps[self.current_step.get()].kind == SequencerStepKind::Skip {
            // If every step is skip just stop
            if needs_skip_check && self.all_steps_skip() {
                self.current_step.set(0);
                return;
            }
            self.increment_step_body(force, false);
        }
    }

    fn all_steps_skip(&self) -> bool {
        for step in self.steps.iter() {
            if step.kind != SequencerStepKind::Skip {
                return false;
            }
        }
        return true; // Also considered true if there are no steps
    }

    pub fn start(&self) {
        self.playing.set(true);
    }

    pub fn stop(&self) {
        self.playing.set(false);
    }

    pub fn set_clock(&mut self, clock: Option<Rc<dyn SynthModule>>) {
        self.clock = clock;
    }

    pub fn set_edge_detection(&mut self, edge_detection: EdgeDetection) {
        self.edge_detection = edge_detection;
    }

    pub fn get_edge_detection(&self) -> EdgeDetection {
        self.edge_detection
    }

    pub fn set_edge_tolerance(&mut self, edge_tolerance: f32) {
        self.edge_tolerance = edge_tolerance;
    }

    pub fn get_edge_tolerance(&self) -> f32 {
        self.edge_tolerance
    }

    pub fn iter(&self) -> std::slice::Iter<StepInfo> {
        self.steps.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<StepInfo> {
        self.steps.iter_mut()
    }

    pub fn into_iter(self) -> std::vec::IntoIter<StepInfo> {
        self.steps.into_iter()
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }
}

impl SynthModule for Sequencer {
    fn fill_output_buffer(&self, data: &mut [f32], output_info: &OutputInfo) {
        let data_size = data.len();

        // Closure to fill the actual data buffer
        // TODO: slide
        let fill_sequencer_buffer = |sequencer: &Self, data: &mut [f32], start: usize, stop: usize| {
            let (step_value, _step_slide) = match sequencer.get_current_step_info() {
                Some(step_info) => (step_info.value, step_info.slide),
                None => (0_f32, 0_f32)
            };

            if stop > data_size {
                // TODO: remove this when I know this is safe
                panic!("Went out of bounds filling sequencer buffer... Probably off-by-one");
            }
            let sub_data = &mut data[start..stop]; // It's quite important that `stop` is < `data_size`
            for datum in sub_data.iter_mut() {
                *datum = step_value;
            }
        };

        if self.playing.get() {
            // We are playing which means which step we are on is subject to change
            let mut clock_signals = Vec::with_capacity(data_size);
            clock_signals.resize(data_size, 0_f32);
            if let Some(clock) = &self.clock {
                clock.fill_output_buffer(&mut clock_signals, output_info);
            }

            let mut data_filled = 0_usize;
            for i in 1..data_size {
                // Step the sequence
                let previous_clock_signal = clock_signals[i - 1];
                let current_clock_signal = clock_signals[i];
                let needs_step = match self.edge_detection {
                    EdgeDetection::Both => 
                        f32::abs(previous_clock_signal - current_clock_signal) > self.edge_tolerance,
                    EdgeDetection::Falling => 
                        current_clock_signal < previous_clock_signal - self.edge_tolerance,
                    EdgeDetection::Rising =>
                        current_clock_signal > previous_clock_signal + self.edge_tolerance
                };
                if needs_step {
                    // Fill what we've passed by with the previous step
                    fill_sequencer_buffer(self, data, data_filled, i);
                    data_filled = i;
                    // Do the increment
                    self.increment_step(false);
                }
            }
            fill_sequencer_buffer(self, data, data_filled, data_size);
        }
        else {
            // We are not playing which means whichever step we're on will fill the whole buffer
            fill_sequencer_buffer(self, data, 0, data_size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::oscillator;
    use crate::prelude::*;
    use crate::clock;

    fn create_output_info(sample_rate: usize, buffer_size: usize) -> OutputInfo {
        let mut clock = clock::SampleClock::new(sample_rate);
        let clock_values = clock.get_range(buffer_size);
        OutputInfo::new_basic(sample_rate, clock_values)
    }

    fn create_test_sequencer() -> Sequencer {
        let mut sequencer = Sequencer::with_steps(5);
        for (i, step) in sequencer.iter_mut().enumerate() {
            step.value = i as f32;
        }

        // Set up clock
        let mut clock_osc = oscillator::Oscillator::new();
        clock_osc.set_frequency(1_f32);
        clock_osc.set_waveform(oscillator::Waveform::Pulse);
        clock_osc.set_pulse_width(0.5);
        sequencer.set_clock(clock_osc.into());

        sequencer
    }

    #[test]
    fn test_remove_step() {
        let mut sequencer = Sequencer::with_steps(5);
        for (i, step) in sequencer.iter_mut().enumerate() {
            step.value = i as f32;
        }

        if let Err(err) = sequencer.remove_step(2) {
            panic!("Failed to remove step 2: {}", err);
        }
        assert_eq!(sequencer.len(), 4, "Expected 4 steps. Got {} steps", sequencer.len());
        for step in sequencer.iter() {
            assert!(!float_eq(step.value, 2.0, 0.0001), "Step 2 is still there after being removed");
        }
    }

    #[test]
    fn test_stopped_output() {
        const SAMPLE_RATE: usize = 9;
        const EXPECTED_DATA: [f32; 9] = [0.0; 9];
        let mut sequencer = create_test_sequencer();

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        // Test output when not playing
        let mut data = Vec::with_capacity(SAMPLE_RATE);
        data.resize(SAMPLE_RATE, 0_f32);
        sequencer.stop();
        sequencer.fill_output_buffer(&mut data, &output_info);
        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], data[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}", EXPECTED_DATA, data
            );
        }
    }

    #[test]
    fn test_stopped_output_after_step() {
        const SAMPLE_RATE: usize = 9;
        const EXPECTED_DATA: [f32; 9] = [1.0; 9];
        let mut sequencer = create_test_sequencer();

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        // Test output when not playing
        let mut data = Vec::with_capacity(SAMPLE_RATE);
        data.resize(SAMPLE_RATE, 0_f32);
        sequencer.stop();
        sequencer.increment_step(true);
        let new_step = sequencer.get_current_step_info().expect("There is no next step?");
        assert!(
            float_eq(new_step.value, 1_f32, 0.0000001),
            "Next step value isn't what I expected. Expected 1.0. Got {}", new_step.value
        ); 

        sequencer.fill_output_buffer(&mut data, &output_info);
        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], data[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}\n", EXPECTED_DATA, data
            );
        }
    }

    #[test]
    fn test_playing_output() {
        const SAMPLE_RATE: usize = 9;
        const EXPECTED_DATA: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 2.0];
        let mut sequencer = create_test_sequencer();

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        // Test output when not playing
        let mut data = Vec::with_capacity(SAMPLE_RATE);
        data.resize(SAMPLE_RATE, 0_f32);
        sequencer.set_edge_detection(EdgeDetection::Both);
        sequencer.start();

        sequencer.fill_output_buffer(&mut data, &output_info);
        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], data[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}\n", EXPECTED_DATA, data
            );
        }
    }

    #[test]
    fn test_playing_skip_steps_output() {
        const SAMPLE_RATE: usize = 9;
        const EXPECTED_DATA: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 2.0, 2.0, 2.0, 2.0, 3.0];
        let mut sequencer = create_test_sequencer();

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        // Test output when not playing
        let mut data = Vec::with_capacity(SAMPLE_RATE);
        data.resize(SAMPLE_RATE, 0_f32);
        sequencer.set_edge_detection(EdgeDetection::Both);
        sequencer.start();

        let step_1 = sequencer.get_step_info_mut(1).expect("There is no step 1?");
        step_1.kind = SequencerStepKind::Skip;

        sequencer.fill_output_buffer(&mut data, &output_info);
        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], data[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}\n", EXPECTED_DATA, data
            );
        }
    }

    #[test]
    fn test_playing_repeat_steps_output() {
        const SAMPLE_RATE: usize = 9;
        const EXPECTED_DATA: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0];
        let mut sequencer = create_test_sequencer();

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        // Test output when not playing
        let mut data = Vec::with_capacity(SAMPLE_RATE);
        data.resize(SAMPLE_RATE, 0_f32);
        sequencer.set_edge_detection(EdgeDetection::Both);
        sequencer.start();

        let step_1 = sequencer.get_step_info_mut(1).expect("There is no step 1?");
        step_1.kind = SequencerStepKind::Repeat;

        sequencer.fill_output_buffer(&mut data, &output_info);
        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], data[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}\n", EXPECTED_DATA, data
            );
        }
    }

    #[test]
    fn test_playing_cycle_output() {
        const SAMPLE_RATE: usize = 9;
        const EXPECTED_DATA: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 4.0, 4.0, 4.0, 4.0, 0.0];
        let mut sequencer = create_test_sequencer();

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        // Test output when not playing
        let mut data = Vec::with_capacity(SAMPLE_RATE);
        data.resize(SAMPLE_RATE, 0_f32);
        sequencer.set_edge_detection(EdgeDetection::Both);
        sequencer.start();

        let step_1 = sequencer.get_step_info_mut(1).expect("There is no step 1?");
        step_1.kind = SequencerStepKind::Skip;
        let step_2 = sequencer.get_step_info_mut(2).expect("There is no step 2?");
        step_2.kind = SequencerStepKind::Skip;
        let step_3 = sequencer.get_step_info_mut(3).expect("There is no step 3?");
        step_3.kind = SequencerStepKind::Skip;

        sequencer.fill_output_buffer(&mut data, &output_info);
        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], data[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}\n", EXPECTED_DATA, data
            );
        }
    }

    #[test]
    fn test_playing_all_steps_skip() {
        const SAMPLE_RATE: usize = 9;
        const EXPECTED_DATA: [f32; 9] = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut sequencer = create_test_sequencer();

        let output_info = create_output_info(SAMPLE_RATE, EXPECTED_DATA.len());

        // Test output when not playing
        let mut data = Vec::with_capacity(SAMPLE_RATE);
        data.resize(SAMPLE_RATE, 0_f32);
        sequencer.set_edge_detection(EdgeDetection::Both);
        sequencer.start();

        for step in sequencer.iter_mut() {
            step.kind = SequencerStepKind::Skip;
        }

        sequencer.fill_output_buffer(&mut data, &output_info);
        for i in 0..SAMPLE_RATE {
            assert!(
                float_eq(EXPECTED_DATA[i], data[i], 0.000001),
                "Output does not match expected.\n\tExpected: {:?}\n\tGot: {:?}\n", EXPECTED_DATA, data
            );
        }
    }
}