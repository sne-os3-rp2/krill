use std::path::PathBuf;
use std::{env, fmt};
use std::process::{Command, Output};
use std::io::Error;
use std::string::FromUtf8Error;
use std::fmt::Display;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct IpfsPath(pub PathBuf);

impl IpfsPath {
    pub fn value(&self) -> &PathBuf {
        &self.0
    }

    pub fn to_string(&self) -> String {
        String::from(&self.0.display().to_string())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RepoPubKey(pub String);

impl RepoPubKey {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl PubKey for RepoPubKey {
    fn key(&self) -> String {
        self.value().clone()
    }
}

pub trait PubKey {
    fn key(&self) -> String;
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TalPubKey(pub String);

impl TalPubKey {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl PubKey for TalPubKey {
    fn key(&self) -> String {
        self.value().clone()
    }
}


const IPFS: &str = "ipfs";

pub fn add(ipfs_path: &IpfsPath, dir: &PathBuf) -> Result<String, Error> {
    env::set_var("IPFS_PATH", ipfs_path.to_string());
    let output = add_dir(dir).unwrap();
    let cid = extract_output_cid(output);
    env::set_var("IPFS_PATH", "");
    return cid;
}

pub fn publish(ipfs_path: &IpfsPath, public_key: &dyn PubKey, cid: String) -> Result<Output, Error> {
    env::set_var("IPFS_PATH", ipfs_path.to_string());
    let result = publish_cid(public_key, &cid);
    env::set_var("IPFS_PATH", "");
    return result
}

pub fn publish_ta_cer(ipfs_path: &IpfsPath, tal_pub_key: &dyn PubKey) -> Result<Output, Error> {
    env::set_var("IPFS_PATH", ipfs_path.to_string());

    let output = Command::new(IPFS)
        .arg("add")
        .arg("/tmp/ta.cer")
        .output()?;

    let cid = extract_output_cid(output.clone())?;
    let result = publish_cid(tal_pub_key, &cid);

    info!("Result of adding cid to ipfs {:?}", &output);
    info!("Result of publishing cid to ipns {:?} is {:?}", &cid, &result);

    env::set_var("IPFS_PATH", "");
    return Ok(output);
}

fn extract_output_cid(output: Output) -> Result<String, Error> {
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

fn publish_cid(public_key: &dyn PubKey, cid: &String) -> Result<Output, Error> {
    let key = format!("--key={}", public_key.key());
    let ipfs_cid = format!("/ipfs/{}", cid);
    Command::new(IPFS)
        .arg("name")
        .arg("publish")
        .arg(key)
        .arg(ipfs_cid)
        .output()
}