#![no_std]
#![no_main]

extern crate alloc;

use alloc::format;
use e_outlet::{Command, parse_command};
use embassy_executor::Spawner;
use embassy_net::{
    Ipv4Address, Runner, StackResources,
    udp::{PacketMetadata, UdpSocket},
};
use embassy_time::{Duration, Instant, Timer, with_timeout};
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_radio::wifi::{
    AuthenticationMethodConfig, Config, ControllerConfig, Interface, WifiController,
    sta::StationConfig,
};

esp_bootloader_esp_idf::esp_app_desc!();

const PORT: u16 = 42423;
const SSID: &str = env!(
    "WIFI_SSID",
    "Set WIFI_SSID in .cargo/esp-config.toml (see example)"
);
const PASSWORD: &str = env!(
    "WIFI_PASSWORD",
    "Set WIFI_PASSWORD in .cargo/esp-config.toml"
);

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default().with_cpu_clock(CpuClock::max()));
    // Physical level: UP = 3.3 V, DOWN = 0 V. Start LOW before initializing Wi-Fi.
    let mut output = Output::new(peripherals.GPIO23, Level::Low, OutputConfig::default());
    let mut up = false;
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);
    let timers = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timers.timer0, peripherals.FROM_CPU_INTR0);

    let interface = Interface::station();
    let mac = interface.mac_address();
    let id = format!(
        "outlet-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    println!("{} GPIO23=DOWN; UDP {}", id, PORT);

    let station = StationConfig::default()
        .with_ssid(SSID.try_into().expect("SSID too long"))
        .with_authentication(AuthenticationMethodConfig::Wpa2Personal(
            PASSWORD.try_into().expect("Wi-Fi password too long"),
        ));
    let controller = WifiController::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(Config::Station(station)),
    )
    .expect("Wi-Fi initialization failed");
    let rng = Rng::new();
    let seed = ((rng.random() as u64) << 32) | rng.random() as u64;
    static RESOURCES: static_cell::StaticCell<StackResources<3>> = static_cell::StaticCell::new();
    let (stack, runner) = embassy_net::new(
        interface,
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );
    spawner.spawn(connection(controller).unwrap());
    spawner.spawn(network(runner).unwrap());

    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx = [0; 1024];
    let mut tx = [0; 1024];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx, &mut tx_meta, &mut tx);
    socket.bind(PORT).expect("UDP bind failed");
    let mut packet = [0; 128];
    let mut next_announce = Instant::now();
    loop {
        if !stack.is_link_up() || !stack.is_config_up() {
            output.set_low();
            up = false;
            // Drop queued commands from the previous connection.
            socket.close();
            stack.wait_config_up().await;
            socket.bind(PORT).expect("UDP rebind failed");
            next_announce = Instant::now();
        }
        if Instant::now() >= next_announce {
            let status = format!(
                "{{\"protocol\":\"EOUTLET/1\",\"id\":\"{}\",\"gpio\":23,\"state\":\"{}\"}}",
                id,
                if up { "up" } else { "down" }
            );
            // Limit sends so a broken network cannot block GPIO handling indefinitely.
            let _ = with_timeout(
                Duration::from_millis(250),
                socket.send_to(status.as_bytes(), (Ipv4Address::BROADCAST, PORT)),
            )
            .await;
            next_announce = Instant::now() + Duration::from_secs(5);
        }
        if let Ok(Ok((len, peer))) =
            with_timeout(Duration::from_millis(250), socket.recv_from(&mut packet)).await
        {
            if !stack.is_link_up() || !stack.is_config_up() {
                continue;
            }
            if let Some(command) = parse_command(&packet[..len], &id) {
                match command {
                    Command::Up => {
                        output.set_high();
                        up = true;
                    }
                    Command::Down => {
                        output.set_low();
                        up = false;
                    }
                    Command::Get => {}
                }
                let status = format!(
                    "{{\"protocol\":\"EOUTLET/1\",\"id\":\"{}\",\"gpio\":23,\"state\":\"{}\"}}",
                    id,
                    if up { "up" } else { "down" }
                );
                let _ = with_timeout(
                    Duration::from_millis(250),
                    socket.send_to(status.as_bytes(), peer),
                )
                .await;
                next_announce = Instant::now();
            }
        }
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    loop {
        match controller.connect_async().await {
            Ok(_) => {
                println!("Wi-Fi connected");
                let _ = controller.wait_for_disconnect_async().await;
                println!("Wi-Fi disconnected");
            }
            Err(error) => println!("Wi-Fi connection error: {:?}", error),
        }
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[embassy_executor::task]
async fn network(mut runner: Runner<'static, Interface>) {
    runner.run().await
}
