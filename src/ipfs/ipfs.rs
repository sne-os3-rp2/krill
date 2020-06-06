use std::path::PathBuf;
use std::env;
use std::process::{Command, Output};
use std::io::Error;

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

fn extract_output_cid(output: Output) -> Result<String, Error>{
    let out_lines = match String::from_utf8(output.stdout) {
        Ok(lines) => lines,
        // TODO revisit error type
        Err(_e) => return Err(Error::from_raw_os_error(1))
    };

    let cid = match out_lines.lines().last() {
        Some(last_line) => last_line.split(" ").collect::<Vec<&str>>()[1],
        // TODO revisit error type
        None => return Err(Error::from_raw_os_error(1))
    };

    return Ok(String::from(cid))
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