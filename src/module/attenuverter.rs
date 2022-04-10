use super::common::{Connectable, SignalOutputModule, OutputInfo};
use super::Empty;

#[derive(Clone)]
pub struct Attenuverter {
    signal_in: Connectable<dyn SignalOutputModule>,
    control_in: Connectable<dyn SignalOutputModule>,
    gain: f32,
    control_gain: f32,
}

impl Attenuverter {
    pub fn new() -> Self {
        let signal_in = Empty::new().into();
        let control_in = Empty::new().into();
        let gain = 0_f32;
        let control_gain = 1_f32;
        Self { signal_in, control_in, gain, control_gain }
    }

    pub fn set_signal_in(&mut self, signal_in: Connectable<dyn SignalOutputModule>) {
        self.signal_in = signal_in;
    }

    pub fn set_control_in(&mut self, control_in: Connectable<dyn SignalOutputModule>) {
        self.control_in = control_in;
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    pub fn set_control_gain(&mut self, control_gain: f32) {
        self.control_gain = control_gain;
    }

    pub fn copy_state_from(&mut self, other: &Self) {
        // Note: Does not update connections
        self.gain = other.gain;
        self.control_gain = other.control_gain;
    }
}

impl SignalOutputModule for Attenuverter {
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo) {
        let buffer_len = buffer.len();

        // Get raw, unattenuated signal
        let mut raw_signal = vec![0.0; buffer_len];
        {
            let mut locked_signal_in = self.signal_in.lock();
            locked_signal_in.fill_output_buffer(&mut raw_signal, output_info);
        }

        // Get control signal
        let mut control = vec![0.0; buffer_len];
        {
            let mut locked_control_in = self.control_in.lock();
            locked_control_in.fill_output_buffer(&mut control, output_info)
        }

        for i in 0..buffer_len {
            let control_datum = control[i];
            let amplitude_factor = 1_f32.min(control_datum + self.gain); // control + gain or 1.0 if > 1
            let attenuverted_datum = raw_signal[i] * amplitude_factor;
            buffer[i] = attenuverted_datum;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::module::sample_buffer::SampleBuffer;
    use crate::clock::SampleClock;
    use super::*;

    const SAMPLE_RATE: usize = 10;

    fn get_constant_signal(amplitude: f32) -> Connectable<dyn SignalOutputModule> {
        let samples = vec![amplitude; SAMPLE_RATE];
        let sample_buffer = SampleBuffer::new(samples);
        sample_buffer.into()
    }

    fn get_attenuverter_output(attenuverter: &mut Attenuverter) -> Vec<f32> {
        let mut clock = SampleClock::new(SAMPLE_RATE);
        let clock_values = clock.get_range(SAMPLE_RATE);
        let output_info = OutputInfo::new_basic(SAMPLE_RATE, clock_values);

        let mut output_buffer = vec![0_f32; SAMPLE_RATE];
        attenuverter.fill_output_buffer(&mut output_buffer, &output_info);
        output_buffer
    }

    #[test]
    fn test_gain() {
        let mut attenuverter = Attenuverter::new();
        attenuverter.set_signal_in(get_constant_signal(1_f32));
        attenuverter.set_gain(0.5);

        let output_buffer = get_attenuverter_output(&mut attenuverter);

        let expected = vec![0.5; SAMPLE_RATE];
        assert_eq!(output_buffer, expected);
    }

    #[test]
    fn test_gain_invert() {
        let mut attenuverter = Attenuverter::new();
        attenuverter.set_signal_in(get_constant_signal(1_f32));
        attenuverter.set_gain(-0.5);

        let output_buffer = get_attenuverter_output(&mut attenuverter);

        let expected = vec![-0.5; SAMPLE_RATE];
        assert_eq!(output_buffer, expected);
    }

    #[test]
    fn test_control() {
        let mut attenuverter = Attenuverter::new();
        attenuverter.set_signal_in(get_constant_signal(1_f32));
        attenuverter.set_control_in(get_constant_signal(0.5));

        let output_buffer = get_attenuverter_output(&mut attenuverter);

        let expected = vec![0.5; 10];
        assert_eq!(output_buffer, expected);
    }

    #[test]
    fn test_gain_and_control() {
        let mut attenuverter = Attenuverter::new();
        attenuverter.set_signal_in(get_constant_signal(1_f32));
        attenuverter.set_control_in(get_constant_signal(0.25));
        attenuverter.set_gain(0.25);

        let output_buffer = get_attenuverter_output(&mut attenuverter);

        let expected = vec![0.5; 10];
        assert_eq!(output_buffer, expected);
    }
}