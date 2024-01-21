use wei_result::*;
use action::*;

mod action;

#[macro_use]
extern crate wei_log;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    wei_windows::init();

    wei_env::bin_init("wei-updater");
    info!("wei-forward start");

    let args: Vec<String> = std::env::args().collect();

    let mut command = "";

    if args.len() > 1 {
        command = &args[1];
    }

    match command {
        "link" => {
            result(link(&args[2], &args[3], &args[4]));
        },
        "link_container" => {
            result(link_container(&args[2], &args[3]));
        }
        "unlink" => {
            result(unlink(&args[2]));
        }
        "start" => {
            result(start());
        },
        "status" => {
            result_value(status());
        },
        "stop" => {
            info!("stop");
            result(wei_run::command_async("wsl", vec![
                "/usr/bin/killall", 
                "frpc"
            ]));
        },
        _ => {
            help();
        }
    }

    Ok(())
}