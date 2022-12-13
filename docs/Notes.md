# Notes

The Google Pixel Buds Pro rely on at least two different protocols apart from the standard audio profiles (HSP/HFP, A2DP, AVRCP):
- The Google Fast Pair Service (GFPS) protocol provides support for somewhat standardized events and actions (next to fast-pairing as advertised in its name).
  This includes battery status of the individual parts (left/right buds and case), multi-point audio source switching notifications, ringing for find-my-device actions, etc.
  See https://developers.google.com/nearby/fast-pair for details.
- The proprietary "Maestro" protocol is used to change settings on the buds (noise-cancelling, equalizer, balance, ...) and likely also update the firmware.

Note that while AVRCP can provide battery information, this only seems to be a single value for both buds combined and does not include the case.
Detailed battery information can only be obtained via the GFPS protocol.


## Google Fast Pair Service Protocol

See https://developers.google.com/nearby/fast-pair for a somewhat limited specification.
Unfortunately this is incomplete.
More details can be found in the Android source code, e.g. [here][gfps-android-0] and [here][gfps-android-1].
The Pixel Buds Pro, however, send additional messages with group and code numbers beyond the ones mentioned there.

The main part of this protocol is a [RFCOMM channel][gfps-rfcomm] which provides events, including battery notifications.
On the Pixel Buds Pro, this also seems to include events for changes to the ANC status (group 8, code 19).

[gfps-android-0]: https://cs.android.com/android/platform/superproject/+/master:out/soong/.intermediates/packages/modules/Connectivity/nearby/tests/multidevices/clients/test_support/fastpair_provider/proto/NearbyFastPairProviderLiteProtos/android_common/xref/srcjars.xref/android/nearby/fastpair/provider/EventStreamProtocol.java;drc=cb3bd7c37d630acb613e10f730c532128a02a3d5;l=69
[gfps-android-1]: https://cs.android.com/android/platform/superproject/+/master:packages/modules/Connectivity/nearby/tests/multidevices/clients/test_support/fastpair_provider/src/android/nearby/fastpair/provider/FastPairSimulator.java;l=1199;drc=cb3bd7c37d630acb613e10f730c532128a02a3d5?q=df21fe2c-2515-4fdb-8886-f12c4d67927c&ss=android%2Fplatform%2Fsuperproject
[gfps-rfcomm]: https://developers.google.com/nearby/fast-pair/specifications/extensions/messagestream


## Maestro Protocol

The "Maestro" protocol is a proprietary protocol for changing settings, used on the Pixel Buds Pro.
It's possible that this is targeted more generally at Google wearable devices.
The protocol not only allows for changing settings or getting hardware/firmware information, but also allows for subscribing to events, such as settings changes.

The protocol is implemented using the [pigweed RPC library](https://pigweed.dev/pw_rpc/), which is similar to [gRPC](https://grpc.io/) and relies on [protocol buffers](https://developers.google.com/protocol-buffers) for message encoding.
In addition, the RPC messages are wrapped in High-Level Data Link Control (HDLC) U-frames (an example for this is given [here](https://pigweed.dev/pw_hdlc/rpc_example/#module-pw-hdlc-rpc-example)).
