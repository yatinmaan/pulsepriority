# PulsePriority

## Build
`cargo build --release`

## Use

 - List all audio outputs sorted by their priority.
     - `pulsepriority -l`

 - Move the device at [index] to highest priority and display the new priority list.
     - `pulsepriority -s [index]`

 - Display (old) priority list, move the device at [index] to highest priority and show the new priority list.
     - `pulsepriority -l -s [index]`

 - Disable priority based routing. (Using -l or -s will automatically re-enable it).
     - `pulsepriority -d`
