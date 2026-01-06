# rsleep

A simple sleep replacement for interactive scripts with live progress display.

### Usage 

`rsleep <duration>[s|m|h|d]`

Supports fractional seconds and suffixes: `m` (minutes), `h` (hours), `d` (days). If no suffix specified - value in seconds

### Return code

After time is over it returns with code 0. Press `q` to quit gracefully (exit 0) or `Ctrl+C` to interrupt (exit 1).

### Build

`cargo build --release`
