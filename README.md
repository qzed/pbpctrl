# `pbpctrl`

Control Google Pixel Buds Pro from the Linux command line. Might or might not work on other Pixel Buds devices.

Allows reading of battery, hardware, software, and runtime information as well as reading and changing settings (ANC state, equalizer, ...). 


## Installation via `cargo`

To build install the binary via cargo, run
```sh
cargo install pbpctrl --git https://github.com/qzed/pbpctrl/
```


## Instructions

Pair and connect your Pixel Buds Pro before use.
Run `pbpctrl help` for more information.


## Notes on Battery Information

The Pixel Buds Pro support basic battery information via the AVCPR standard.
Support for this is still experimental in BlueZ and needs to be enabled manually by editing `/etc/bluetooth/main.conf` and setting
```
[General]
Experimental = true
```
or by starting BlueZ with the `--experimental` flag.
After this, battery status should be provided via UPower.

Note that this, however, will only provide a single battery meter for both buds combined, and none for the case.
For more detailed information, use `pbpctrl show battery`.
This also allows reading of the case battery as long as one bud is placed in the case (note that the case does not have a Bluetooth receiver itself).


## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
