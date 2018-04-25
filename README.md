RingBuf
=====

[![Build Status](https://travis-ci.org/HyeonuPark/ringbuf.svg?branch=master)](https://travis-ci.org/HyeonuPark/ringbuf)
[![codecov](https://codecov.io/gh/HyeonuPark/ringbuf/branch/master/graph/badge.svg)](https://codecov.io/gh/HyeonuPark/ringbuf)

Thread safe bounded channels based on ring buffer.

This crate provides channels that can be used to communicate
between asynchronous tasks.

Channels are based on fixed-sized ring buffer. Send operations simply fail
if backing buffer is full, and you can get back message with error.

## License

This repository is dual-licensed under the [MIT license][license-mit]
and [Apache license 2.0][license-apl] at your option.
By contributing to RingBuf you agree that your contributions will be licensed
under these two licenses.

<!-- links -->

[license-mit]: ./LICENSE-MIT
[license-apl]: ./LICENSE-APACHE
