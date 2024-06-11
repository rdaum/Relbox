# Relbox

## What is this?

An experimental in-memory database you shouldn't trust your data with.

## Why?

This was built mostly as internals for another project of mine, but I'm making it a standalone crate
to separate the development process.

## What's in it?

Combines a few things:

  * Transactional quasi-binary-relational structures based around versioned copy-on-write trees,
    of a few different kinds, including:
      * im::HashMap for hash indexes
      * My own Adaptive Radix Tree implementation for ordered indexes  
  * Buffer pool management in the style of Umbra and LeanStore.
  * Simple and probably buggy page storage using io_uring.
  * Uses the `okaywal` crate right now to manage a write-ahead log, but the plan is to
    roll my own.

There's a small battery of tests that make an attempt to verify some integrity, and this thing has
had a little testing with a small amount of data, but it's not ready for production use,
and there *are bugs*.  

Don't go using it. But you're welcome to contribute.

## Long term intent?

To store binary relational data quickly and efficiently, with a focus on being able to
retrieve and update it without going through a query language layer.

To apply some interesting things in DB research papers I've read.

To be able to use it as a backend for a few other projects I have in mind.

## License?

GPL v3.0

