use super::super::error::{ModuleError, ModuleResult};
use super::super::common::{MutexPtr, NoteOutputModule, OutputInfo, OutputTimestamp};
use super::super::midi::MidiModuleBase;
use crate::midi;
use crate::note::{Note, NoteInterval};

use std::collections::HashSet;

pub struct MidiNoteOutput {
    midi_source: MutexPtr<MidiModuleBase>,
    active_notes: HashSet<u8>
}

impl MidiNoteOutput {
    pub fn new(midi_source: MutexPtr<MidiModuleBase>) -> Self {
        let on_notes = HashSet::new();
        Self { midi_source, active_notes: on_notes }
    }

    // Gets all notes that are currently on
    pub fn get_notes_on_absolute(&self) -> ModuleResult<HashSet<u8>> {
        let midi_src = match self.midi_source.lock() {
            Ok(midi_src) => midi_src,
            Err(err) => {
                let msg = format!("Failed to get notes from MIDI file. Lock is poisoned!: {}", err);
                return Err(ModuleError::new(&msg));
            }
        };

        midi_src.get_notes_on_absolute()
    }
    
    /// Gets changes in note state since the last time this was called
    fn consume_notes_on_off_delta(
        &mut self, n_microseconds: usize, timestamp: &OutputTimestamp
    ) -> ModuleResult<midi::data::NoteDelta> {
        let mut midi_src = match self.midi_source.lock() {
            Ok(midi_src) => midi_src,
            Err(err) => {
                let msg = format!("Failed to get notes from MIDI file. Lock is poisoned!: {}", err);
                return Err(ModuleError::new(&msg));
            }
        };

        midi_src.consume_notes_on_off_delta(n_microseconds, timestamp)
    }

    fn get_active_notes(&self) -> &HashSet<u8> {
        &self.active_notes
    }
}

impl NoteOutputModule for MidiNoteOutput {
    fn get_output(&mut self, n_samples: usize, output_info: &OutputInfo) -> Vec<NoteInterval> {
        // TODO: This does not take retriggers into account. In a normal synth if a note went off and on again
        // at the same instant the envelope would be retriggered. But that doesn't happen here... 
        let mut midi_source_lock = match self.midi_source.lock() {
            Ok(midi_source_lock) => midi_source_lock,
            Err(err) => {
                // TODO: Remove panic. I think that involves changing the signature of MidiMonoNoteOutput
                panic!("MIDI source lock is poisoned!: {}", err);
            }
        };

        // Some timing stuff
        // (n_samples / sample_rate) * 1,000,000
        let sample_period_microseconds = n_samples * 1_000_000 / output_info.sample_rate;
        let start_microseconds = midi_source_lock.get_time();
        let microseconds_per_sample = sample_period_microseconds / n_samples;

        let note_delta = match midi_source_lock.consume_notes_on_off_delta(
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

            //let sample_num = (delta_start_microseconds - start_microseconds) / microseconds_per_sample;
            match delta.get_event_type() {
                midi::data::NoteEventType::On => {
                    // When a note turns on we always create a new interval.
                    // The interval will always have `None` as a end sample. We'll
                    // fill it in if we see an end event
                    self.active_notes.insert(delta.get_note_number());
                    let note = Note::from_midi_note(delta.get_note_number());
                    let interval = NoteInterval::new(note, Some(sample_num), None);
                    intervals.push(interval);
                }
                midi::data::NoteEventType::Off => {
                    // When a note turns off we find its corresponding interval and add a end sample.
                    // This should always find and interval to end. If it doesn't then something is wrong.
                    self.active_notes.remove(&delta.get_note_number());
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::common::OutputTimestamp;
    use crate::util::test_util;

    use core::panic;
    use std::sync::{Arc, Mutex};

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
        
        let arc_mutex_midi = Arc::new(Mutex::new(midi_module_base));
        MidiNoteOutput::new(arc_mutex_midi)
    }

    #[test]
    fn get_notes_delta() {
        let mut midi_module = get_test_midi_module();
        let mut midi_source_lock = midi_module.midi_source.lock().expect("Failed to lock midi source");
        midi_source_lock.set_channel(Some(0));
        drop(midi_source_lock);

        let delta = match midi_module.consume_notes_on_off_delta(10_000_000, &OutputTimestamp::empty()) {
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
        let mut midi_source_lock = midi_module.midi_source.lock().expect("Failed to lock midi source");
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