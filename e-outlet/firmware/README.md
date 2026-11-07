# E-outlet — ESP32 Rust firmware

Classic ESP32 WROOM-32, Rust `no_std`, esp-hal + esp-radio + Embassy.
GPIO23 is an output: `up` = HIGH (3.3 V), `down` = LOW (0 V).

## 1. Install Rust and the ESP32 tools

Install Rust using [rustup](https://rustup.rs/). On Windows, use the MSVC toolchain
and install the Visual Studio C++ build tools if requested by the Rust installer.
In a terminal, install the tools (versions used for this project):

```sh
rustup update stable
cargo +stable install espup --version 0.19.0 --locked
espup install --targets esp32
cargo +stable install espflash --version 4.7.0 --locked
```

`espup` installs the Xtensa toolchain named `esp`. `espflash` programs the board and
displays serial output. The project already exists: neither `esp-generate` nor
`cargo-espflash` is needed to build or flash it.

On Windows, close and reopen the terminal after installation to pick up the
environment variables added by espup. No PowerShell script is required.

On Linux/macOS, load the environment in each new terminal:

```sh
. "$HOME/export-esp.sh"
```

On Debian/Ubuntu, build prerequisites include `gcc`, `build-essential`, `curl`
and `pkg-config`.

### Existing installation on this Windows PC

Executables from the initial setup are also available in `.tools/bin`.
To use them in a PowerShell window without reinstalling or running a script:

```powershell
$env:PATH = "$PWD\.tools\bin;$env:USERPROFILE\.rustup\toolchains\esp\xtensa-esp-elf\bin;$env:PATH"
```

### Windows USB driver

If the CP2102 board does not appear as a COM port, install the
[Silicon Labs CP210x Universal Windows Driver](https://www.silabs.com/software-and-tools/usb-to-uart-bridge-vcp-drivers).
Extract the ZIP, then run this from the extracted directory in an **administrator** terminal:

```powershell
pnputil /add-driver ".\silabser.inf" /install
```

Unplug and reconnect the board. Device Manager error code 28 means the driver is
not installed.

## 2. Configure Wi-Fi

Run all commands below from the `firmware` directory.
During the initial setup, copy the template **only if the local file does not already exist**:

```powershell
# Windows / PowerShell
Copy-Item .cargo\esp-config.example.toml .cargo\esp-config.toml
notepad .cargo\esp-config.toml
```

Or in Bash:

```bash
cp .cargo/esp-config.example.toml .cargo/esp-config.toml
```

Enter the credentials for a 2.4 GHz WPA2 Personal network:

```toml
[env]
WIFI_SSID = "your-wifi"
WIFI_PASSWORD = "your-password"
```

This file is ignored by Git. Credentials are embedded in the compiled firmware.
After changing the Wi-Fi settings, rebuild and flash the firmware again.

## 3. Build

```sh
cargo +esp build --release --locked
```

The binary is `target/xtensa-esp32-none-elf/release/e-outlet`.
`Cargo.lock` pins the Rust dependencies.

## 4. Flash and open the serial monitor

Connect the board using a USB data cable, then list the ports:

```sh
espflash list-ports
```

On Windows (replace COM3 with the detected port):

```powershell
espflash flash --chip esp32 --port COM3 --monitor target/xtensa-esp32-none-elf/release/e-outlet
```

On Linux/macOS, use the same command with the appropriate port:

```sh
espflash flash --chip esp32 --port /dev/ttyUSB0 --monitor target/xtensa-esp32-none-elf/release/e-outlet
```

On macOS, the port may be `/dev/cu.usbserial-…`. If connecting to the bootloader
fails, hold BOOT when flashing starts, then release it once connected.
Ctrl+C exits the monitor. To reopen it without flashing:

```sh
espflash monitor --port COM3
```

At startup, the board prints its name followed by `Wi-Fi connected`.

## 5. Listen and send commands with Bash

One script: `control.sh`. Requirements: **Bash and Python 3**, with no extra Python packages.
On Windows, use Git Bash with Python 3 installed on Windows; on Linux/macOS, use a
Bash terminal. If Python is not found, install [Python 3](https://www.python.org/downloads/)
and make it available in PATH. Select a specific interpreter with `PYTHON=python`.

Listen for announcements until Ctrl+C, or for 15 seconds:

```bash
bash control.sh listen
bash control.sh listen --seconds 15
```

Each line contains the source IP and received JSON. The board announces its name and
state every 5 seconds. The PC must be on the same local network; allow Python on
private networks if Windows Firewall prompts you. A VPN or Wi-Fi client isolation
may prevent communication.

Send commands directly to the board's IP (values observed on our board):

```bash
bash control.sh get  192.0.2.10 outlet-0123456789ab
bash control.sh up   192.0.2.10 outlet-0123456789ab
bash control.sh down 192.0.2.10 outlet-0123456789ab
```

The IP may change after reconnection: use `listen` to find it again. The name remains
stable because it comes from the MAC address. Commands use unicast, which worked in
the hardware test; the earlier broadcast command did not receive a reply.
The script waits for an acknowledgement matching the name and requested state,
makes up to three attempts, and returns a nonzero exit code on failure.

## Protocol

UDP, port **42423**. UTF-8 text commands:

```text
EOUTLET/1 outlet-0123456789ab GET
EOUTLET/1 outlet-0123456789ab UP
EOUTLET/1 outlet-0123456789ab DOWN
```

Unicast reply to the source port, and broadcast announcement to `255.255.255.255:42423`:

```json
{"protocol":"EOUTLET/1","id":"outlet-0123456789ab","gpio":23,"state":"down"}
```

Invalid packets and packets addressed to another name are ignored. UP/DOWN are
idempotent: retries do not reverse the command. `state` describes the commanded
level, without relay feedback. GPIO23 returns LOW at startup and when the network
stack detects loss of the Wi-Fi link or IP configuration; Wi-Fi reconnects automatically.
Controller silence does not reset the output to LOW.
An active-low relay requires adapting the logic to its hardware interface.
The protocol has no authentication: use it on a trusted local network.

Validated on the WROOM-32: build, flash on COM3, Wi-Fi connection, UDP announcement
and UP command with acknowledgement. The Bash script still needs to be tried on the board.

Official documentation: [espup](https://github.com/esp-rs/espup),
[espflash](https://github.com/esp-rs/espflash/tree/main/espflash),
[Rust ESP toolchain installation](https://docs.espressif.com/projects/rust/book/getting-started/toolchain.html).
