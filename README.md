# `pbpctrl`

Control Google Pixel Buds Pro from the Linux command line.


## Installation via `cargo`

To build install the binary via cargo, run
```sh
cargo install pbpctrl --git https://github.com/qzed/pbpctrl/
```


## Instructions

Pair and connect your Pixel Buds Pro before use.
Run `pbpctrl help` for more infos.


## Notes on Battery Information

The Pixel Buds Pro support basic battery information via the AVCPR standard.
Support for this is still experimental in bluez and needs to be enabled manually by editing `/etc/bluetooth/main.conf` and setting
```
[BR]
Experimental = true
```
or by starting bluez with the `--experimental` flag.

Note that this, however, will only provide a single battery meter for both buds combined, and none for the case.
For more detailed information, use `pbpctrl show battery`.
This also allows reading of the case battery as long as one bud is placed in the case (note that the case does not have a Bluetooth receiver itself).
