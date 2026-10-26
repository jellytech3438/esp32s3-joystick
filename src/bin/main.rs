#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::timer::timg::TimerGroup;
use esp_println::{self as _, println};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO0
    // - GPIO3
    // - GPIO45
    // - GPIO46
    // These GPIO pins are in use by some feature of the module and should not be used.
    let mut adc2_config = AdcConfig::new();
    let _gpio13 = peripherals.GPIO13;
    let _gpio14 = peripherals.GPIO14;
    let _gpio18 = peripherals.GPIO18;

    let mut x_pin = adc2_config.enable_pin(_gpio13, Attenuation::_11dB);
    let mut y_pin = adc2_config.enable_pin(_gpio14, Attenuation::_11dB);

    let mut adc2 = Adc::new(peripherals.ADC2, adc2_config);

    let mut button = Input::new(_gpio18, InputConfig::default().with_pull(Pull::Up));

    // only printout while x or y beyond certain threshold
    let mut prev_x = 0;
    let mut prev_y = 0;
    let mut prev_btn = false;
    let mut print_vals = true;

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0, peripherals.FROM_CPU_INTR0);

    let _ = spawner;

    loop {
        Timer::after(Duration::from_secs(1)).await;

        let Ok(vrx): Result<_, _> = nb::block!(adc2.read_oneshot(&mut x_pin)) else {
            continue;
        };
        let Ok(vry): Result<_, _> = nb::block!(adc2.read_oneshot(&mut y_pin)) else {
            continue;
        };

        if vrx.abs_diff(prev_x) > 100 {
            prev_x = vrx;
            print_vals = true;
        }
        if vry.abs_diff(prev_y) > 100 {
            prev_y = vry;
            print_vals = true;
        }

        if print_vals {
            print_vals = false;
            println!("X: {}, Y: {}", vrx, vry);
        }

        let button_state = button.is_low();
        if button_state && !prev_btn {
            println!("Button pressed");
            print_vals = true;
        }
        prev_btn = button_state;
    }
}
