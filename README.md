# Hardware projects

Design sources, firmware, and fabrication exports for the electronics projects in this repository:

- `e-outlet/`: motorized outlet, enclosure, PCB, and ESP32 firmware. See the firmware README for setup and control commands.
- `irrigator/`: irrigation circuit designs and notes.
- `motordriver/`: motor driver circuits, documentation, and photographs.

Open KiCad projects with their project files and the included libraries. The outlet symbol library uses a project-relative path.

## Firmware configuration

Copy `e-outlet/firmware/.cargo/esp-config.example.toml` to `esp-config.toml` in the same directory and configure it locally. The real configuration, build outputs, and local tools are ignored. Compiled firmware can embed Wi-Fi credentials; build your own and do not publish binaries containing private settings.

Device identifiers and the `192.0.2.10` address in the control examples are placeholders. Replace them with your device's actual values.

## Fabrication exports

The CNC `.nc` files and sliced `.3mf` files are machine-specific reference exports. Their original machine and slicer profiles have not been documented or validated for other equipment. Inspect units, origin, toolpaths, clearances, tooling, feeds, speeds, printer settings, and materials; regenerate for your own equipment before use.
