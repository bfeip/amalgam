use super::super::OutputInfo;
use super::super::midi::MidiModuleBase;
use crate::SynthResult;
use crate::midi;
use crate::midi::data::NoteDelta;
use crate::module::SynthModule;
use crate::note::{Note, NoteInterval};

use std::collections::HashSet;
use std::rc::Rc;
use std::time::Instant;
use std::cell::{RefCell, Ref};

pub struct MidiNoteOutput {
    midi_source: Rc<MidiModuleBase>,
    active_notes: RefCell<HashSet<u8>>
}

impl MidiNoteOutput {
    pub fn new(midi_source: Rc<MidiModuleBase>) -> Self {
        let on_notes = RefCell::new(HashSet::new());
        Self { midi_source, active_notes: on_notes }
    }

    // Gets all notes that are currently on
    pub fn get_notes_on_absolute(&self) -> SynthResult<HashSet<u8>> {
        self.midi_source.get_notes_on_absolute()
    }
    
    /// Gets changes in note state since the last time this was called
    fn read_notes_on_off_delta(
        &self, n_microseconds: usize, timestamp: &Instant
    ) -> SynthResult<NoteDelta> {
        self.midi_source.read_notes_on_off_delta(n_microseconds, timestamp)
    }

    fn get_active_notes(&self) -> Ref<HashSet<u8>> {
        self.active_notes.borrow()
    }

    fn read_note_intervals(&self, n_samples: usize, output_info: &OutputInfo) -> Vec<NoteInterval> {
        // TODO: This does not take retriggers into account. In a normal synth if a note went off and on again
        // at the same instant the envelope would be retriggered. But that doesn't happen here... 

        // Some timing stuff
        // (n_samples / sample_rate) * 1,000,000
        let sample_period_microseconds = n_samples * 1_000_000 / output_info.sample_rate;
        let start_microseconds = self.midi_source.get_time();
        let microseconds_per_sample = sample_period_microseconds / n_samples;

        let note_delta = match self.midi_source.read_notes_on_off_delta(
            sample_period_microseconds,
            &output_info.timestamp
        ) {
            Ok(delta) => delta,
            Err(err) => {
                // TODO: Remove panic. I think that involves changing the signature of MidiMonoNoteOutput
                panic!("Failed to get MIDI notes delta: {}", err);
            }
        };

        if note_delta.is_empty() {
            // There aren't any changes for this sample period.
            // Do whatever we were doing before.
            let active_midi_notes = self.get_active_notes();
            let mut intervals = Vec::with_capacity(active_midi_notes.len());
            for active_midi_note in active_midi_notes.iter().cloned() {
                let active_note = Note::from_midi_note(active_midi_note);
                let interval = NoteInterval::new(active_note, None, None);
                intervals.push(interval);
            }
            return intervals;
        }
        
        let mut intervals = Vec::new();
        for active_note in self.get_active_notes().iter().cloned() {
            // Get initially active notes and make intervals for all of them.
            // Intervals created here have no start sample since it started in a previous sample period.
            // They also have no end sample initially though one might be added if a note off event is seen
            // in this sample period.
            let note = Note::from_midi_note(active_note);
            let interval = NoteInterval::new(note, None, None);
            intervals.push(interval);
        }

        for delta in note_delta.iter() {
            let delta_start_microseconds = delta.get_time_in_microseconds(note_delta.get_ticks_per_second());
            // Note: Due to precision loss issues in MIDI it's very possible for this to be negative. If it is,
            // just play it immediately
            let sample_num = delta_start_microseconds.saturating_sub(start_microseconds) / microseconds_per_sample;

            match delta.get_event_type() {
                midi::data::NoteEventType::On => {
                    // When a note turns on we always create a new interval.
                    // The interval will always have `None` as a end sample. We'll
                    // fill it in if we see an end event
                    let note_number = delta.get_note_number();
                    let mut active_notes = self.active_notes.borrow_mut();
                    debug_assert!(
                        !active_notes.contains(&note_number),
                        "Activated a note we were already playing"
                    );
                    active_notes.insert(note_number);

                    let note = Note::from_midi_note(delta.get_note_number());
                    let interval = NoteInterval::new(note, Some(sample_num), None);
                    intervals.push(interval);
                }
                midi::data::NoteEventType::Off => {
                    // When a note turns off we find its corresponding interval and add a end sample.
                    // This should always find and interval to end. If it doesn't then something is wrong.
                    let successfully_removed = self.active_notes.borrow_mut().remove(&delta.get_note_number());
                    debug_assert!(successfully_removed, "Tried to remove an active note that didn't exist");

                    let note = Note::from_midi_note(delta.get_note_number());

                    for interval in intervals.iter_mut() {
                        if interval.note == note && interval.end.is_none() {
                            interval.end = Some(sample_num);
                            break;
                        }
                    }
                }
            }
        }
        intervals
    }
}

impl SynthModule for MidiNoteOutput {
    fn fill_output_buffer(&self, buffer: &mut [f32], output_info: &OutputInfo) {
        // Put whatever the first interval is as the output until it's done then move on to the second, etc.
        let fill_with_interval = |buffer: &mut [f32], interval: NoteInterval, current_sample: usize| {
            let end = interval.end.unwrap_or(buffer.len());
            let signal_out = interval.note.to_freq_normalized();
            buffer[current_sample..end].fill(signal_out);
        };

        let intervals = self.read_note_intervals(buffer.len(), output_info);
        let mut current_sample = 0_usize;
        for interval in intervals {
            if interval.start.unwrap_or(0) <= current_sample {
                fill_with_interval(buffer, interval, current_sample)
            }
            else {
                buffer[current_sample..interval.start.unwrap()].fill(0.0);
                fill_with_interval(buffer, interval, interval.start.unwrap());
            }
            current_sample = interval.end.unwrap_or(buffer.len());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::common::OutputTimestamp;
    use crate::util::test_util;

    use core::panic;

    fn get_test_midi_module() -> MidiNoteOutput {
        let path = test_util::get_test_midi_file_path();
        let mut midi_module_base = match MidiModuleBase::open(path) {
            Ok(midi_module_base) => midi_module_base,
            Err(err) => {
                panic!("Failed to get midi module base: {}", err);
            }
        };

        if let Err(err) = midi_module_base.set_track(1) {
            panic!("Failed to set to correct track: {}", err);
        }
        
        MidiNoteOutput::new(midi_module_base.into())
    }

    #[test]
    fn get_notes_delta() {
        let mut midi_module = get_test_midi_module();
        let mut midi_source_lock = midi_module.midi_source.get().unwrap();
        midi_source_lock.set_channel(Some(0));
        drop(midi_source_lock);

        let delta = match midi_module.read_notes_on_off_delta(10_000_000, &OutputTimestamp::empty()) {
            Ok(delta) => delta,
            Err(err) => {
                panic!("Failed to get note delta: {}", err);
            }
        };
        assert_ne!(delta.delta.len(), 0, "Expected to get notes back");

        let mut notes_on = HashSet::<u8>::new();
        for note_event in delta.delta {
            if note_event.get_event_type() == midi::data::NoteEventType::On {
                notes_on.insert(note_event.get_note_number());
            }
            else if note_event.get_event_type() == midi::data::NoteEventType::Off {
                notes_on.remove(&note_event.get_note_number());
            }
        }
        assert_eq!(notes_on.len(), 0, "Expected every note that was on to have an off counterpart");
    }

    #[test]
    fn get_notes_on_absolute() {
        let midi_module = get_test_midi_module();

        let target_microseconds = 5_000_000; // Just trust me bro. It'll have three notes on
        let mut midi_source_lock = midi_module.midi_source.get().unwrap();
        midi_source_lock.set_time(target_microseconds);
        midi_source_lock.set_channel(Some(0));
        drop(midi_source_lock);
        
        let notes_on = match midi_module.get_notes_on_absolute() {
            Ok(notes_on) => notes_on,
            Err(err) => {
                panic!("Failed to get notes on: {}", err);
            }
        };

        assert_eq!(notes_on.len(), 3, "Expected there to be three notes on");
        assert!(notes_on.contains(&55) &&
                notes_on.contains(&52) &&
                notes_on.contains(&48),
                "Expected notes 55, 52 & 73 to be on"
        );
    }

    #[test]
    fn get_active_notes() {
        let on_notes: HashSet<u8> = HashSet::from([1, 4, 3, 5, 2]);
        let mut midi_module = get_test_midi_module();
        midi_module.active_notes = on_notes.clone();
        
        let active_notes = midi_module.get_active_notes();
        assert_eq!(*active_notes, on_notes);
    }
}