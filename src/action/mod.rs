use crate::CMD;
use crate::FRP;
// use serde_json::Value;
// use wei_result::*;

pub fn start() -> Result<(), Box<dyn std::error::Error>> {
    // 判断 wsl ls /frpc.toml 是否存在，如果不存在，则创建
    
    // match wei_run::command(CMD, vec!["ls", "/frpc.toml"]) {
    //     Ok(_) => {},
    //     Err(_) => {
    //         write_conf(&conf())?;
    //     }
    // };

    write_conf(&conf())?;

    wei_run::command(CMD, vec!["mkdir", "-p", "/frpc/"])?;

    #[cfg(not(target_os = "windows"))]
    wei_run::command(CMD, vec!["killall", "frpc"])?;

    wei_run::command(CMD, vec![
        FRP, 
        "-c", 
        "/frpc.toml"
    ])?;

    Ok(())
}

pub fn write_conf(data: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file_name = format!("./frpc.toml");
    std::fs::write(file_name.as_str(), data)?;

    match wei_run::command(CMD, vec!["mv", "./frpc.toml", "/frpc.toml"]) {
        Ok(_) => {},
        Err(_) => {}
    };

    // 删除本地文件
    match std::fs::remove_file(file_name.as_str()) {
        Ok(_) => {},
        Err(_) => {}
    };

    Ok(())
}

pub fn write_link_conf(file_name: &str, data: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("write_link_conf: {}", file_name);

    std::fs::write(file_name, data)?;

    wei_run::command(CMD, vec!["mkdir", "-p", "/frpc/"])?;
    wei_run::command(CMD, vec!["mv", file_name, "/frpc/"])?;

    match std::fs::remove_file(file_name) {
        Ok(_) => {},
        Err(_) => {}
    };

    Ok(())
}

pub fn conf() -> String {
r#"[common] 
token = "D487DD0B55DFCACDD920CCBDAEAFA351"
server_addr = "xlai.cc" 
server_port = 7000
protocol = "kcp"
admin_addr = "0.0.0.0" 
admin_port = 7401
log_file = "/tmp/frpc.log"

includes = "/frpc/*.toml"
"#.to_string()
}

pub fn manager_one_file(data: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 获取 frp 列表
    // 列出 container- 开头的
    // 列出 wei-docker container 列表
    // 如果 frp 列表中的 容器 不存在 docker 列表中，则删除 frp 列表中的配置
    // 扫描 frp 列表中的 container- 开头的配置，再去 wei-docker ip container_name 获取 ip，如果 ip 有变动，则更新 frp 配置

    let root_value: toml::Value = toml::from_str(data).expect("Failed to parse the file");

    for (key,value) in root_value.as_table().unwrap() {
        if key.starts_with("container-") {
            let container_name = key.split("-").collect::<Vec<&str>>()[1];
            let container_data = wei_run::run("wei-docker", vec!["container_ip", &container_name])?;
            let container_data: serde_json::Value = serde_json::from_str(&container_data)?;
            let ip = container_data["data"].as_str().ok_or("")?;
            if ip != "" {
                let local_ip = value["local_ip"].as_str().ok_or("")?;
                let local_port = format!("{}", value["local_port"]);
                let remote_port = format!("{}", value["remote_port"]);
                if local_ip != ip {
                    link_container(&container_name, &local_port, &remote_port)?;
                }
            }
        }
    }


    Ok(())
}

pub fn manager() -> Result<(), Box<dyn std::error::Error>> {
    let output = wei_run::command(CMD, vec!["ls", "/frpc"])?;
    let output = output.split("\n").collect::<Vec<&str>>();

    for file in output {
        if file != "" {
            let file_path = format!("/frpc/{}", file);
            let data = wei_run::command(CMD, vec!["cat", &file_path])?;
            manager_one_file(&data)?;
        }
    }

    Ok(())
}

pub fn link_container(container_name: &str, port: &str, remote_port: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("link_container: {}, {}", container_name, port);
    let data = wei_run::run("wei-docker", vec!["container_ip", container_name])?;
    let data: serde_json::Value = serde_json::from_str(&data)?;
    let ip = data["data"].as_str().ok_or("")?;

    if ip == "" {
        return Err("container ip is empty".into());
    }
    let container_name = format!("container-{}", container_name);

    link(&container_name, ip, port, remote_port)
}

pub fn link(name: &str, ip: &str, port: &str, remote_port: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("link: {}, {}, {}, {}", name, ip, port, remote_port);
    // let root_string = read_conf()?;

    // 请求服务器获取 frp 服务器地址，如果远程服务器不可用，则使用默认穿透服务器 xlai.cc 及默认key
    // let common_str: String = match reqwest::blocking::get("http://download.zuiyue.com/forward/index.html") {
    //     Ok(res) => res.text()?,
    //     Err(_) => conf()
    // };

    let file_name = format!("{}.toml", name);
    let file_path = format!("/frpc/{}", file_name);
    let root_string = wei_run::command(CMD, vec!["cat", &file_path])?;

    let mut root_value: toml::Value = match toml::from_str(&root_string) {
        Ok(v) => v,
        Err(_) => toml::Value::Table(toml::value::Table::new()),
    };

    // link的参数有二个，ip， 端口号
    let link_string = r#"
[{name}-{port}-{remote_port}-{uuid}]
type = "tcp"
local_ip = "{ip}"
local_port = {port}
remote_port = {remote_port}
"#.replace("{name}", name)
    .replace("{ip}", ip)
    .replace("{port}", port)
    .replace("{remote_port}", remote_port)
    .replace("{uuid}", &wei_api::uuid());

    let link_value: toml::Value = toml::from_str(&link_string).unwrap();

    // 将 link_value 中的每个元素加入到 root_value 中
    if let Some(root_table) = root_value.as_table_mut() {
        if let Some(link_table) = link_value.as_table() {
            for (key, value) in link_table {
                root_table.insert(key.clone(), value.clone());
            }
        }
    }

    let root_string = toml::to_string(&root_value).unwrap();
    
    write_link_conf(&file_name, &root_string)?;
    reload()?;

    info!("save finish");
    Ok(())
}

pub fn status() -> Result<String, Box<dyn std::error::Error>> {
    let url = "http://localhost:7401/api/status";

    let data = ureq::get(url).call()?.into_string()?;

    Ok(data)
}

pub fn unlink(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = wei_run::command(CMD, vec!["ls", "/frpc"])?;
    let output = output.split("\n").collect::<Vec<&str>>();

    for file in output {
        if file.starts_with(name) {
            let file_path = format!("/frpc/{}", file);
            wei_run::command(CMD, vec!["rm", &file_path])?;
        }
    }

    Ok(())
}

// pub fn read_conf() -> Result<String, Box<dyn std::error::Error>> {
//     let mut root_string = wei_run::command(CMD, vec!["cat", "/frpc.toml"])?;

//     if root_string.contains("No such file or directory") {
//         root_string = conf();
//     }

//     Ok(root_string)
// }

pub fn reload() -> Result<(), Box<dyn std::error::Error>> {
    info!("reload");
    wei_run::command(CMD, vec![
        FRP, 
        "reload",
        "-c",
        "/frpc.toml"
    ])?;
 
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
