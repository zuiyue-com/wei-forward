#[macro_use]
extern crate wei_log;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    wei_env::bin_init("wei-updater");
    use single_instance::SingleInstance;
    let instance = SingleInstance::new("wei-updater").unwrap();
    if !instance.is_single() { 
        std::process::exit(1);
    };

    Ok(())
}