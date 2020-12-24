use super::traits::SignalOutputModule;
use super::error::*;
use super::empty::Empty;

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
pub enum SequencerEdgeDetection {
    Rising,
    Falling,
    Both
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
    playing: bool,
    cycle: bool,
    current_step: usize,

    clock: Box<dyn SignalOutputModule>,
    edge_detection: SequencerEdgeDetection,
    edge_tolerance: f32
}

impl Sequencer {
    pub fn new() -> Self {
        let steps = Vec::new();
        let playing = false;
        let cycle = true;
        let current_step = 0_usize;

        let clock = Box::new(Empty::new());
        let edge_detection = SequencerEdgeDetection::Falling;
        let edge_tolerance = 0.8_f32;
        Self { steps, playing, cycle, current_step, clock, edge_detection, edge_tolerance }
    }

    pub fn with_steps(step_count: usize) -> Self {
        let mut steps = Vec::with_capacity(step_count);
        for _ in 0..step_count {
            steps.push(DEFAULT_STEP_INFO);
        }

        let playing = false;
        let cycle = true;
        let current_step = 0_usize;

        let clock = Box::new(Empty::new());
        let edge_detection = SequencerEdgeDetection::Falling;
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
        self.steps.get(self.current_step)
    }

    pub fn set_step_info(&mut self, step_index: usize, step_info: &StepInfo) -> ModuleResult<()> {
        match self.steps.get_mut(step_index) {
            Some(step) => *step = *step_info,
            None => {
                let msg = "Failed to set sequencer step info because index was out of bounds";
                return Err(ModuleError::new(msg));
            }
        }
        Ok(())
    }

    pub fn remove_step(&mut self, step_index: usize) -> ModuleResult<()> {
        if step_index > self.steps.len() {
            let msg = "Failed to remove sequencer step because index is out of bounds";
            return Err(ModuleError::new(msg));
        }

        self.steps.remove(step_index);
        Ok(())
    }

    pub fn increment_step(&mut self, force: bool) {
        self.increment_step_body(force, true);
    }

    // This is the recursive component of increment_step. it has the additional
    // parameter of needs_skip_check to note weather we need to check for the 
    // case where all steps are skip so we don't have to iterate every step
    // every time there's a skip step.
    fn increment_step_body(&mut self, force: bool, needs_skip_check: bool) {
        let sequence_length = self.steps.len();
        if sequence_length == 0 {
            // There are no steps, bail
            return;
        }
        if !force && !self.cycle && self.current_step == sequence_length - 1 {
            // We're on the last step and we're not cycling and it's not being forced, do nothing
            return;
        }

        // Set us to the next step
        if self.steps[self.current_step].kind == SequencerStepKind::Repeat {
            self.current_step = 0;
        } else {
            self.current_step = self.current_step + 1;
            if self.current_step % sequence_length == 0 {
                if !self.cycle {
                    self.playing = false;
                }
            }
        }

        // Check if the step we're on now is a skipped step. If it is, recurse
        if self.steps[self.current_step].kind == SequencerStepKind::Skip {
            // If every step is skip just stop
            if needs_skip_check && self.all_steps_skip() {
                self.playing = false;
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

    pub fn start(&mut self) {
        self.playing = true;
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }

    pub fn set_clock(&mut self, clock: Box<dyn SignalOutputModule>) {
        self.clock = clock;
    }
}

impl SignalOutputModule for Sequencer {
    fn fill_output_buffer(&mut self, data: &mut [f32]) {
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

        if self.playing {
            // We are playing which means which step we are on is subject to change
            let mut clock_signals = Vec::with_capacity(data_size);
            clock_signals.resize(data_size, 0_f32); // NOTE: initializing this vec isn't really needed, but is safe
            self.clock.fill_output_buffer(&mut clock_signals);
            let mut data_filled = 0_usize;
            for i in 1..data_size {
                // Step the sequence
                let previous_clock_signal = clock_signals[i - 1];
                let current_clock_signal = clock_signals[i];
                let needs_step = match self.edge_detection {
                    SequencerEdgeDetection::Both => 
                        f32::abs(previous_clock_signal - current_clock_signal) > self.edge_tolerance,
                    SequencerEdgeDetection::Falling => 
                        current_clock_signal < previous_clock_signal - self.edge_tolerance,
                    SequencerEdgeDetection::Rising =>
                        current_clock_signal > previous_clock_signal + self.edge_tolerance
                };
                if needs_step {
                    // Fill what we've passed by with the previous step
                    fill_sequencer_buffer(self, data, data_filled, i + 1);
                    data_filled = i + 1;
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