use super::common::*;
use super::mixer::Mixer;
use crate::note::{Note, Tone};

pub trait Voice: Send + Clone {
    /// Called when a voice is activated with the note that it should play.
    fn on_activate(&mut self, note: Note);

    /// Called when a voice is deactivated. This might, for example, Set envelopes to begin the release
    /// stage. The voice can still produce audio.
    fn on_start_deactivate(&mut self);

    /// Checks to see if the voice is fully done playing. E.g There's no more audio
    /// Used to know if the voice can be re-activated with a new note
    fn fully_deactivated(&self) -> bool;

    /// Called to update a voice to match a reference voice. This should not, for example, reset
    /// envelopes or change the note that oscillators recieve.
    fn update(&mut self, reference_voice: &Self);

    /// Retrive a `MutexPtr` for whatever the final output module for the voice is.
    /// E.g. if it's a pair of oscillators into a filter, this should retrive a mutable ref to the filter.
    fn get_end_module(&mut self) -> MutexPtr<dyn SignalOutputModule>;
}

impl<T: Voice> SignalOutputModule for T {
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo) {
        self.get_end_module().lock().expect("Failed to lock voice output").fill_output_buffer(buffer, output_info);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum VoiceStatus {
    Activated,
    Deactivateing,
    Deactived
}

struct VoiceState<V: Voice> {
    voice: V,
    note: Note,
    status: VoiceStatus
}

struct VoiceSet<V: Voice, N: NoteOutputModule> {
    reference_voice: MutexPtr<V>,
    max_voices: usize,
    voice_states: Vec<VoiceState<V>>,

    note_source: MutexPtr<N>
}

impl<V: Voice, N: NoteOutputModule> VoiceSet<V, N> {
    /// Creates a new voice box.
    /// Pass 0 for max voices for no limit
    fn new(reference_voice: MutexPtr<V>, max_voices: usize, note_source: MutexPtr<N>) -> Self {
        // Create voices
        let mut voice_states = Vec::with_capacity(max_voices);
        {
            // Lock block
            let reference_voice_lock = reference_voice.lock().expect("Reference voice lock is poisoned");
            for _ in 0..max_voices {
                let voice = reference_voice_lock.clone();
                let note = Note::new(4, Tone::C);
                let status = VoiceStatus::Deactived;
                let voice_state = VoiceState { voice, note, status };
                voice_states.push(voice_state)
            }
        }

        Self { reference_voice, max_voices, voice_states, note_source }
    }
}

impl<V: Voice, N: NoteOutputModule> SignalOutputModule for VoiceSet<V, N> {
    fn fill_output_buffer(&mut self, buffer: &mut [f32], output_info: &OutputInfo) {
        let buffer_len = buffer.len();
        let active_notes_by_sample = {
            // TODO: Remove expect
            let mut note_source = self.note_source.lock().expect("Voice box note_source lock is poisoned");
            note_source.get_output(buffer_len, output_info)
        };

        let reference_voice = self.reference_voice.lock().expect("Reference voice lock was poisoned");

        for i in 0..buffer_len {
            let notes_that_should_be_active = &active_notes_by_sample[i];
            let mut initially_active_notes = Vec::with_capacity(self.voice_states.len());

            // Mark any voices that are done playing as deactivated and start
            // deactivating any that need to be deactivated. And also update any
            // voices that are producing audio to match the reference voice
            for voice_state in self.voice_states.iter_mut() {
                if voice_state.status == VoiceStatus::Activated && !notes_that_should_be_active.contains(
                    &voice_state.note
                ) {
                    // Voice is playing but we should stop it
                    voice_state.voice.on_start_deactivate();
                    voice_state.status = VoiceStatus::Deactivateing;
                }

                if voice_state.status == VoiceStatus::Deactivateing && voice_state.voice.fully_deactivated() {
                    // Voice is completly done playing
                    voice_state.status = VoiceStatus::Deactived
                }
                else if voice_state.status == VoiceStatus::Activated {
                    initially_active_notes.push(voice_state.note);
                }

                if voice_state.status != VoiceStatus::Deactived {
                    // Update any voices that are making audio to reflect changes to the reference voice
                    voice_state.voice.update(&reference_voice);
                }
            }

            // Activate any new voices
            for note in notes_that_should_be_active {
                if !initially_active_notes.contains(&note){
                    // A note is being triggered.
                    let mut new_voice = reference_voice.clone();
                    new_voice.on_activate(*note);

                    let new_voice_state = VoiceState{
                        voice: new_voice,
                        note: *note,
                        status: VoiceStatus::Activated
                    };

                    match self.voice_states.iter_mut().find(|v| v.status == VoiceStatus::Deactived) {
                        Some(free_voice) => {
                            *free_voice = new_voice_state;
                        },
                        None => {
                            // All voices are active or deactivating
                            // TODO: Search again for deactivating voices and replace one of them
                            if self.max_voices == 0 {
                                // Unlimited voices... Let's just make a new one
                                self.voice_states.push(new_voice_state);
                            }
                        }
                    }
                }
            }

            // Compute the audio using a mixer... It's easier that way
            let mut mixer = Mixer::with_inputs(notes_that_should_be_active.len());
            for (mixer_input, voice_state) in mixer.iter_inputs_mut().zip(self.voice_states.iter_mut()) {
                if voice_state.status != VoiceStatus::Deactived {
                    let voice_signal_output_module = voice_state.voice.get_end_module();
                    mixer_input.set_input(voice_signal_output_module)
                }
            }
            mixer.fill_output_buffer(buffer, output_info)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::oscillator::Oscillator;
    use crate::clock::SampleClock;

    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct TestVoice {
        osc: MutexPtr<Oscillator>,
        active: bool
    }

    impl TestVoice {
        fn new() -> Self {
            let osc = Arc::new(Mutex::new(Oscillator::new()));
            Self { osc, active: false }
        }
    }

    impl Voice for TestVoice {
        fn on_activate(&mut self, note: Note) {
            self.osc.lock().expect("Osc lock is poisoned").set_frequency(note.to_freq());
            self.active = true;
        }

        fn on_start_deactivate(&mut self) {
            self.active = false;
        }

        fn fully_deactivated(&self) -> bool {
            !self.active
        }

        fn update(&mut self, reference: &Self) {
            let ref_osc = reference.osc.lock().expect("Reference Osc lock is poisoned");
            let mut osc = self.osc.lock().expect("Osc lock is poisoned");

            let ref_state = ref_osc.get_state();
            let mut state = osc.get_state_mut();
            state.pulse_width = ref_state.pulse_width;
            state.waveform = ref_state.waveform;
        }

        fn get_end_module(&mut self) -> MutexPtr<dyn SignalOutputModule> {
            self.osc.clone()
        }
    }

    struct TestNoteSource {
        notes: Vec<Note>,
        send_interval: Vec<bool>
    }

    impl TestNoteSource {
        fn new(notes: &[Note], n_samples: usize) -> Self {
            let send_interval = vec![true; n_samples];
            Self { notes: notes.to_owned(), send_interval }
        }

        fn set_send_interval(&mut self, interval: &[bool]) {
            self.send_interval = interval.to_owned();
        }
    }

    impl NoteOutputModule for TestNoteSource {
        fn get_output(&mut self, n_samples: usize, _output_info: &OutputInfo) -> Vec<Vec<Note>> {
            assert_eq!(n_samples, self.send_interval.len(), "What?");
            let mut ret = Vec::with_capacity(self.notes.len());
            for send in self.send_interval.iter().cloned() {
                if send {
                    ret.push(self.notes.clone());    
                }
                else {
                    ret.push(Vec::new());
                }
            }
            ret
        }

        fn fill_output_buffer(&mut self, buffer: &mut [Vec<Note>], output_info: &OutputInfo) {
            let output = self.get_output(buffer.len(), output_info);
            for (datum, value) in buffer.iter_mut().zip(output) {
                *datum = value;
            }
        }
    }

    fn create_test_output_info(sample_rate: usize) -> OutputInfo {
        let mut sample_clock = SampleClock::new(sample_rate);
        let sample_range = sample_clock.get_range(sample_rate);
        OutputInfo::new(sample_rate, sample_range, OutputTimestamp::empty())
    }

    #[test]
    fn get_output_with_limited_voices() {
        const NOTES: &[Note] = &[
            Note::new(1, Tone::A),
            Note::new(2, Tone::B),
            Note::new(3, Tone::C)
        ];
        let ref_voice = TestVoice::new();
        let note_source = TestNoteSource::new(NOTES, 100);
        let mut voice_set = VoiceSet::new(Arc::new(Mutex::new(ref_voice)), 5, Arc::new(Mutex::new(note_source)));

        let output_info = create_test_output_info(100);
        let mut output_buffer = [0_f32; 100];
        voice_set.fill_output_buffer(&mut output_buffer, &output_info);

        assert_ne!(output_buffer[0], 0.0, "Expected some actual values");

        // TODO: Maybe we could check that the output has the frequencies we expect via DFT?
    }

    #[test]
    fn get_output_with_limited_voices_maxed_out() {
        const NOTES: &[Note] = &[
            Note::new(1, Tone::A),
            Note::new(2, Tone::B),
            Note::new(3, Tone::C)
        ];
        let ref_voice = TestVoice::new();
        let note_source = TestNoteSource::new(NOTES, 100);
        let mut voice_set = VoiceSet::new(Arc::new(Mutex::new(ref_voice)), 1, Arc::new(Mutex::new(note_source)));

        let output_info = create_test_output_info(100);
        let mut output_buffer = [0_f32; 100];
        voice_set.fill_output_buffer(&mut output_buffer, &output_info);

        assert_ne!(output_buffer[0], 0.0, "Expected some actual values");

        // TODO: Maybe we could check that the output has the frequencies we expect via DFT?
    }

    #[test]
    fn get_output_with_unlimited_voices() {
        const NOTES: &[Note] = &[
            Note::new(1, Tone::A),
            Note::new(2, Tone::B),
            Note::new(3, Tone::C)
        ];
        let ref_voice = TestVoice::new();
        let note_source = TestNoteSource::new(NOTES, 100);
        let mut voice_set = VoiceSet::new(Arc::new(Mutex::new(ref_voice)), 0, Arc::new(Mutex::new(note_source)));

        let output_info = create_test_output_info(100);
        let mut output_buffer = [0_f32; 100];
        voice_set.fill_output_buffer(&mut output_buffer, &output_info);

        assert_ne!(output_buffer[0], 0.0, "Expected some actual values");

        // TODO: Maybe we could check that the output has the frequencies we expect via DFT?
    }
}