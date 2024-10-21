pub mod tasks;

use cu29::clock::CuDuration;
use cu29_derive::copper_runtime;
use cu29_helpers::basic_copper_setup;
use cu29_log_derive::debug;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::Duration;

const PREALLOCATED_STORAGE_SIZE: Option<usize> = Some(1024 * 1024 * 100);

#[copper_runtime(config = "copperconfig.ron")]
struct HelloWorldApplication {}

fn run_loop(
    application: &mut HelloWorldApplication,
    clock: cu29::clock::RobotClock,
) -> Result<(), Box<dyn std::error::Error>> {
    static STOP_FLAG: AtomicBool = AtomicBool::new(false);

    ctrlc::set_handler(move || {
        println!("Ctrl-C pressed. Stopping all tasks...");
        STOP_FLAG.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let loop_start_time = clock.now();

    while !STOP_FLAG.load(Ordering::SeqCst)
        && (clock.now() - loop_start_time) < CuDuration::from(Duration::from_millis(2))
    {
        application.run_one_iteration()?;
    }

    application.stop_all_tasks()?;
    debug!("End of app: final clock: {}.", clock.now());

    Ok(())
}

fn main() {
    let logger_path = "hello-world.copper";
    let copper_ctx =
        basic_copper_setup(&PathBuf::from(logger_path), PREALLOCATED_STORAGE_SIZE, true)
            .expect("Failed to setup logger.");
    debug!("Logger created at {}.", logger_path);
    let clock: cu29::clock::RobotClock = copper_ctx.clock;
    let mut application =
        HelloWorldApplication::new(clock.clone(), copper_ctx.unified_logger.clone())
            .expect("Failed to create runtime.");
    debug!("Running... starting clock: {}.", clock.now());

    run_loop(&mut application, clock.clone()).expect("Failed to run application.");

    debug!("End of program: {}.", clock.now());
    sleep(Duration::from_secs(1));
}
