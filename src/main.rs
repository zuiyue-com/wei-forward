#[cfg(target_os = "windows")]
static DATA_1: &'static [u8] = include_bytes!("../../wei-release/windows/san/san.txt");

use serde_json::{json, Value};

#[macro_use]
extern crate wei_log;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    if std::env::args().collect::<Vec<_>>().len() > 1000 {
        println!("{:?}", DATA_1);
    }

    wei_env::bin_init("wei-updater");
    info!("wei-forward start");

    let args: Vec<String> = std::env::args().collect();

    let mut command = "";

    if args.len() > 1 {
        command = &args[1];
    }

    match command {
        "link" => {
            info!("link");
            if args.len() < 4 {
                print!("{}", json!({
                    "code": 400,
                    "message": "参数错误，wei-forward link <ip> <port>"
                }));
                return Ok(());
            }
            let ip = &args[2];
            let port = &args[3];
            link(ip, port)?;
        },
        "link_container" => {
            info!("link_container");
            if args.len() < 4 {
                print!("{}", json!({
                    "code": 400,
                    "message": "参数错误，wei-forward link_container <container_name> <port>"
                }));
                return Ok(());
            }
            let ip = &args[2];
            let port = &args[3];
            link(ip, port)?;
        }
        "unlink" => {
            info!("unlink");
            if args.len() < 4 {
                print!("{}", json!({
                    "code": 400,
                    "message": "参数错误，wei-forward unlink <ip> <port>"
                }));
                return Ok(());
            }
            let ip = &args[2];
            let port = &args[3];
            unlink(ip, port)?;
        }
        "start" => {
            info!("start");
            wei_run::command_async("wsl", vec![
                "/root/data/frp/frpc", 
                "-c", 
                "/root/data/frp/frpc.toml"
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

            let body_value: Value = serde_json::from_str(&body)?;

            println!("{}", json!({
                "code": 200,
                "message": "success",
                "data" : body_value
            }));
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

fn link(ip: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root_string: String = ureq::get("http://localhost:7400/api/config")
        .call()?.into_string()?;

    // 请求服务器获取 frp 服务器地址，如果远程服务器不可用，则使用默认穿透服务器 xlai.cc 及默认key
    let common_str: String = match ureq::get("http://download.zuiyue.com/forward/index.html").call() {
        Ok(res) => {
            res.into_string()?
        },
        Err(_) => r#"
            [common] 
            token = "D4875DFCACDD920CCBDAEAFA351" 
            server_addr = "xlai.cc" 
            server_port = 7000 
            protocol = "kcp"
            admin_addr = "0.0.0.0" 
            admin_port = 7400
        "#.to_string()
    };

    let mut root_value: toml::Value = toml::from_str(&root_string).expect("Failed to parse the file");

    // 删除 common 节点
    root_value.as_table_mut().unwrap().remove("common");

    // 解析服务器的 common_str 为 toml::Value
    let common_value: toml::Value = toml::from_str(&common_str).unwrap();

    // 将 common_value 中的每个元素加入到 root_value 中
    if let Some(root_table) = root_value.as_table_mut() {
        if let Some(common_table) = common_value.as_table() {
            for (key, value) in common_table {
                root_table.insert(key.clone(), value.clone());
            }
        }
    }

    // link的参数有二个，ip， 端口号
    let link_string = r#"
        [link-{name}-{port}]
        type = "tcp"
        local_ip = "{ip}"
        local_port = {port}
        remote_port = 0
    "#.replace("{name}", &ip.replace(".", "_"))
    .replace("{ip}", ip)
    .replace("{port}", port);
  
    let link_value: toml::Value = toml::from_str(&link_string).unwrap();

    // 将 link_value 中的每个元素加入到 root_value 中
    if let Some(root_table) = root_value.as_table_mut() {
        if let Some(link_table) = link_value.as_table() {
            for (key, value) in link_table {
                root_table.insert(key.clone(), value.clone());
            }
        }
    }
    
    save(root_value)?;
    Ok(())
}

fn unlink(ip: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root_string: String = ureq::get("http://localhost:7400/api/config")
        .call()?.into_string()?;

    
    let mut root_value: toml::Value = toml::from_str(&root_string).expect("Failed to parse the file");

    let remove_table = format!("link-{}-{}", &ip.replace(".", "_"), port);
    root_value.as_table_mut().unwrap().remove(&remove_table);

    save(root_value)?;
    Ok(())
}

fn save(root_value: toml::Value) -> Result<(), Box<dyn std::error::Error>> {
    // 将 root_value 转换为 toml 字符串, 并put http://localhost:7400/api/config
    let root_string = toml::to_string(&root_value).unwrap();

    let response = ureq::put("http://localhost:7400/api/config")
        .send_string(&root_string);

    // 检查响应状态
    if !response.ok().is_some() {
        print!("{}", json!({
            "code": 400,
            "message": "save config error"
        }));
        return Ok(());
    }

    let response = ureq::get("http://localhost:7400/api/reload").call();

    match response {
        Ok(_) => {
            print!("{}", json!({
                "code": 200,
                "message": "success"
            }));
            return Ok(());
        },
        Err(_) => {
            print!("{}", json!({
                "code": 400,
                "message": "reload error"
            }));
        }
    }
    Ok(())
}

fn help() {
    println!("wei-forward open <ip> <port>");
    println!("wei-forward start");
    println!("wei-forward stop");
}

fn _print_toml(val: &toml::Value, prefix: String) {
    match val {
        toml::Value::String(s) => println!("{} = {:?}", prefix, s),
        toml::Value::Integer(i) => println!("{} = {:?}", prefix, i),
        toml::Value::Float(f) => println!("{} = {:?}", prefix, f),
        toml::Value::Boolean(b) => println!("{} = {:?}", prefix, b),
        toml::Value::Datetime(dt) => println!("{} = {:?}", prefix, dt),
        toml::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                _print_toml(v, format!("Array: {}[{}]", prefix, i));
            }
        }
        toml::Value::Table(tbl) => {
            for (k, v) in tbl.iter() {
                _print_toml(v, format!("{}.{}", prefix, k));
            }
        }
    }
}