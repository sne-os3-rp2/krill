use std::path::PathBuf;
use std::env;
use std::process::{Command, Output};
use std::io::Error;
use std::string::FromUtf8Error;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IpfsPath {
    pub path: PathBuf
}


impl IpfsPath {
    pub fn from_path_buff(path: PathBuf) -> Self {
        IpfsPath {path}
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IpnsPubkey {
    pub key: String
}

const IPFS: &str = "ipfs";

pub fn add(ipfs_path: &IpfsPath, dir: &PathBuf) -> String {
    // TODO change to return result
    env::set_var("IPFS_PATH", ipfs_path.path.display().to_string());
    let output = add_dir(dir).unwrap();
    let cid = extract_output_cid(output).unwrap();
    env::set_var("IPFS_PATH", "");
    return cid;
}

pub fn publish(ipfs_path: &IpfsPath, public_key: &IpnsPubkey, cid: String) -> Result<Output, Error> {
    env::set_var("IPFS_PATH", ipfs_path.path.display().to_string());
    let result = publish_cid(public_key, &cid);
    env::set_var("IPFS_PATH", "");
    return result
}

pub fn start_ipfs_daemon(ipfs_path_buf: &PathBuf) {
    let mut repo_lock = ipfs_path_buf.clone();
    repo_lock.push("repo.lock");

    if !repo_lock.as_path().exists() {
        println!("Starting IPFS...");
        env::set_var("IPFS_PATH", ipfs_path_buf.as_os_str());
        Command::new(IPFS).arg("daemon")
            .arg("--enable-namesys-pubsub")
            .arg("--enable-pubsub-experiment")
            .spawn()
            .expect("Could not start IPFS");
        println!("Started IPFS...");
        env::set_var("IPFS_PATH", "");
    };
}


fn extract_output_cid(output: Output) -> Result<String, Error>{
    let result:Result<String, FromUtf8Error> = String::from_utf8(output.stdout);
    result.map(move |res| {
        String::from(res.lines()
            .last()
            .unwrap_or(String::from("").as_ref())
            .split(" ")
            .collect::<Vec<&str>>()[1])
    }).map_err(|_e| Error::from_raw_os_error(1))
}


fn add_dir(dir: &PathBuf) -> Result<Output, Error> {
    Command::new(IPFS).arg("add")
        .arg("-r")
        .arg(dir.display().to_string())
        .output()
}

fn publish_cid(public_key: &IpnsPubkey, cid: &String) -> Result<Output, Error> {
    let key = format!("--key={}", public_key.key);
    let ipfs_cid = format!("/ipfs/{}", cid);
    Command::new(IPFS)
        .arg("name")
        .arg("publish")
        .arg(key)
        .arg(ipfs_cid)
        .output()
}