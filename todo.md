# TODO

## Right Now
- How MIDI notes are read needs a tune up for sure. Especially since MIDI notes should no longer come out of a module
- Oscillator needs to not rely on a supplied sample range. The calculation should be immutable and only need to rely on the playback time.
- There should be only a single error and result type for the entire crate.

## Docs and comments and cleanup
There's some docs but I really just need to run through the whole codebase and doc everything. While I'm at it I should gather up the the `// TODO` comments and put them here so they don't get forgotten about.

## Filters
I really just need to sit down and start working on this FFT stuff. No reading books first. No doing my own FFT. Just pick a crate and implement a low pass filter as best I can.

## Split MIDI crate
The MIDI functionality is kinda relatively complete. It could be split into a separate crate and then I could just publish that separately.

It might need some tests though.