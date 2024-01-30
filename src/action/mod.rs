use crate::CMD;
use serde_json::Value;
use wei_result::*;

pub fn start() -> Result<(), Box<dyn std::error::Error>> {
    // 判断 wsl ls /frpc.toml 是否存在，如果不存在，则创建
    let output = wei_run::command(CMD, vec!["ls", "/frpc.toml"])?;

    if output.contains("No such file or directory") {
        write_conf(&conf())?;
    }

    wei_run::command_async(CMD, vec![
        "/usr/bin/frpc", 
        "-c", 
        "/frpc.toml"
    ])?;

    Ok(())
}

pub fn write_conf(data: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file_name = format!("./frpc.toml");
    std::fs::write(file_name.as_str(), data)?;

    wei_run::command(CMD, vec!["mv", "./frpc.toml", "/frpc.toml"])?;

    // 删除本地文件
    match std::fs::remove_file(file_name.as_str()) {
        Ok(_) => {},
        Err(_) => {}
    };

    Ok(())
}

pub fn conf() -> String {
r#"
[common] 
token = "D487DD0B55DFCACDD920CCBDAEAFA351"
server_addr = "xlai.cc" 
server_port = 7000
protocol = "kcp"
admin_addr = "0.0.0.0" 
admin_port = 7400
"#.to_string()
}

pub fn manager() -> Result<(), Box<dyn std::error::Error>> {
    // 获取 frp 列表
    // 列出 container- 开头的
    // 列出 wei-docker container 列表
    // 如果 frp 列表中的 容器 不存在 docker 列表中，则删除 frp 列表中的配置
    // 扫描 frp 列表中的 container- 开头的配置，再去 wei-docker ip container_name 获取 ip，如果 ip 有变动，则更新 frp 配置

    let data = status()?;
    
    let forward_list = data["tcp"].as_array().ok_or("")?;

    for i in forward_list {
        let name = i["name"].as_str().ok_or("")?;
        if name.starts_with("container-") {
            let container_name = name.split("-").collect::<Vec<&str>>()[1];
            let container_data = wei_run::run("wei-docker", vec!["container_ip", &container_name])?;
            let container_data: serde_json::Value = serde_json::from_str(&container_data)?;
            let ip = container_data["data"].as_str().ok_or("")?;
            if ip != "" {
                let local_ip = i["local_ip"].as_str().ok_or("")?;
                if local_ip != ip {
                    // unlink(&name)?;
                    link(&name, ip, i["local_port"].as_str().ok_or("")?)?;
                }
            }
        }
    }

    Ok(())
}

pub fn link_container(container_name: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("link_container: {}, {}", container_name, port);
    let data = wei_run::run("wei-docker", vec!["container_ip", container_name])?;
    let data: serde_json::Value = serde_json::from_str(&data)?;
    let ip = data["data"].as_str().ok_or("")?;

    if ip == "" {
        return Err("container ip is empty".into());
    }
    let container_name = format!("container-{}", container_name);

    link(&container_name, ip, port)
}

pub fn link(name: &str, ip: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("link: {}, {}, {}", name, ip, port);
    let url = "http://localhost:7400/api/config";

    let root_string = match reqwest::blocking::get(url) {
        Ok(res) => res.text()?,
        Err(e) => {
            error(format!("connect frp api error: {}", e));
            std::process::exit(0);
        }
    };

    // 请求服务器获取 frp 服务器地址，如果远程服务器不可用，则使用默认穿透服务器 xlai.cc 及默认key
    let common_str: String = match reqwest::blocking::get("http://download.zuiyue.com/forward/index.html") {
        Ok(res) => res.text()?,
        Err(_) => conf()
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
        [{name}-{port}]
        type = "tcp"
        local_ip = "{ip}"
        local_port = {port}
        remote_port = 0
    "#.replace("{name}", name)
    // .replace("{ip_name}", &ip.replace(".", "_"))
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
    
    info!("save");
    save(root_value)?;
    info!("save finish");
    Ok(())
}

pub fn status() -> Result<Value, Box<dyn std::error::Error>> {
    let url = "http://localhost:7400/api/status";
    let body: String = match reqwest::blocking::get(url) {
        Ok(res) => res.text()?,
        Err(e) => {
            error(format!("connect frp api error: {}", e));
            std::process::exit(0);
        }
    };

    let body_value: Value = serde_json::from_str(&body)?;

    Ok(body_value)
}

pub fn unlink(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = "http://localhost:7400/api/config";

    let root_string: String = match reqwest::blocking::get(url) {
        Ok(res) => res.text()?,
        Err(e) => {
            error(format!("connect frp api error: {}", e));
            std::process::exit(0);
        }
    };

    let mut root_value: toml::Value = toml::from_str(&root_string).expect("Failed to parse the file");

    let remove_table = format!("{}", &name);
    root_value.as_table_mut().unwrap().remove(&remove_table);

    save(root_value)?;
    Ok(())
}

pub fn save(root_value: toml::Value) -> Result<(), Box<dyn std::error::Error>> {
    // 将 root_value 转换为 toml 字符串, 并put http://localhost:7400/api/config
    let root_string = toml::to_string(&root_value)?;

    info!("put");
    reqwest::blocking::Client::new()
        .put("http://localhost:7400/api/config")
        .body(root_string)
        .send()?;

    info!("reload");
    reqwest::blocking::Client::new()
        .get("http://localhost:7400/api/reload")
        .send()?;

    info!("reload finish");
    Ok(())
}

pub fn _print_toml(val: &toml::Value, prefix: String) {
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
