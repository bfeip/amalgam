# TODO

## Right Now
- Fill out Module enum
- Remove Optional and Note output modules. Modules should only output signals
    - Anything else is not a module
- We might want to handle errors in the audio loop when we try to look up a modules in the module manager but can't find it
- How MIDI notes are read needs a tune up for sure.

## Public API rework
The current API is very heavy handed and exposes some logic that ought to be internal, particularly with respect to threading. I should redesign this. Special care needs to be taken about polyphony. Maybe we want to undo that work and completely re-implement it since currently it requires a whole bunch of bullshit.

### How it currently works
- The user creates a `Synth` object and an `AudioOutput` object
    - They should not have to create both, I know there's a reason why they're separate that has to do with the audio thread but... It's just annoying
- The user creates their modules and is required to wrap them in a `Connectable` type wrapper
    - A `Connectable` is a wrapper around an `Option<Arc<Mutex<T>>>`
    - This is to abstract away the threading logic mostly. Every module is in a `Send` and `Sync`package "neatly" The problem is that this logic is exposed to the user. It should be invisible.
- The modules are then hooked up the the synth's output module. Which I think is different from the `AudioOutput` object we created earlier.
- The synth itself then has to be wrapped into a `Arc<Mutex<Synth>>`
    - Again, annoying. I really want to get rid of all this threading logic that is user facing.
- Then the `Synth::play` method is called with a reference to the audio output

### How it should work
- User creates a top level `Synth` object.
    - This should also like, create the audio output object. I don't care how I need to re-jigger the threading problems. It's unreasonable that these should be created separately.
- User creates some modules. I think modules either ought to be registered in the synth, or created through it.
    - This will be because the `Synth` object should maintain ownership of all the modules
    - I think the user should straight up create the modules so they can configure them and then pass them along to the synth. Whereupon they'll receive a handle in return
- Play is called on the `Synth` object to start the playback loop.

## Docs and comments and cleanup
There's some docs but I really just need to run through the whole codebase and doc everything. While I'm at it I should gather up the the `// TODO` comments and put them here so they don't get forgotten about.

## Filters
I really just need to sit down and start working on this FFT stuff. No reading books first. No doing my own FFT. Just pick a crate and implement a low pass filter as best I can.

## Split MIDI crate
The MIDI functionality is kinda relatively complete. It could be split into a separate crate and then I could just publish that separately.

It might need some tests though.