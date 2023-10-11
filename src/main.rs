#[macro_use]
extern crate wei_log;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    wei_env::bin_init("wei-updater");

    info!("wei-forward start");

    let args: Vec<String> = std::env::args().collect();

    let mut command = "";

    if args.len() > 1 {
        command = &args[1];
    }

    match command {
        "open" => {
            info!("open");
            open();
        },
        "start" => {
            info!("start");
            wei_run::command_async("wsl", vec![
                "/root/data/frp/frpc", 
                "-c", 
                "/root/data/frp/frpc.ini"
            ])?;

            print!("{}", serde_json::json!({
                "code": 200,
                "message": "success"
            }));
        },
        "status" => {
            info!("status");
            let body: String = ureq::get("http://localhost:7400/api/status")
                .call()?
                .into_string()?;

            println!("{}", body);
        },
        "stop" => {
            info!("stop");
            
        },
        _ => {
            help();
        }
    }

    Ok(())
}

fn open() {

}

fn help() {
    println!("wei-forward open <ip> <port>");
    println!("wei-updater start");
    println!("wei-updater stop");
}