use anyhow::Result;
use std::collections::HashMap;
use reqwest::Client;
use std::sync::Arc;
use tokio::fs;
use crate::validator_vamp::config::IpfsConfig;

#[derive(Debug, Clone)]
pub struct IpfsService {
    config: IpfsConfig,
    client: Arc<Client>,
}
impl IpfsService {
    pub fn new(config: &IpfsConfig) -> Self {
        Self {
            config: config.clone(),
            client: Arc::new(Client::new()),
        }
    }

    /// Publishes a map of JSON strings keyed by address to IPFS under a common directory path.
    /// Returns the root CID of the directory and a mapping of individual file CIDs.
    pub async fn publish_balance_map(
        &self,
        intent_path: &String,
        entry_by_oth_address: &HashMap<String, String>,
    ) -> Result<(String, HashMap<String, String>)> {
        let dir = tempfile::tempdir()?;
        let mut cid_by_oth_address = HashMap::new();

        for (addr, json_string) in entry_by_oth_address {
            let file_path = dir.path().join(format!("{}.json", addr));
            fs::write(&file_path, json_string).await?;
        }

        let mut form = reqwest::multipart::Form::new();
        
        // Add each file to the form
        for entry in std::fs::read_dir(dir.path())? {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.is_file() {
                let file_name = file_path.file_name().unwrap().to_string_lossy();
                let content = std::fs::read(&file_path)?;
                let part = reqwest::multipart::Part::bytes(content).file_name(file_name.to_string());
                form = form.part("file", part);
            }
        }

        let url = format!(
            "{}/api/v0/add?recursive=true&wrap-with-directory=true&pin=true",
            self.config.api_url
        );

        let res = self.client
            .post(&url)
            .multipart(form)
            .send()
            .await?
            .text()
            .await?;

        log::info!("IPFS add response: {}", res);

        let mut root_cid = None;

        for line in res.lines() {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                let name = entry["Name"].as_str().unwrap_or_default();
                let hash = entry["Hash"].as_str().unwrap_or_default().to_string();
                if name.ends_with(".json") {
                    if let Some(addr) = name.strip_suffix(".json") {
                        cid_by_oth_address.insert(addr.to_string(), hash);
                    }
                } else {
                    root_cid = Some(hash);
                }
            }
        }

        let root = root_cid.ok_or_else(|| anyhow::anyhow!("Root CID not found in IPFS output"))?;

        // Optional MFS copy
        let mfs_path = format!("/vamp/fun/validated/{}", intent_path);
        let mfs_url = format!(
            "{}/api/v0/files/cp?arg=/ipfs/{}&arg={}&parents=true",
            self.config.api_url, root, mfs_path
        );
        let _ = self.client.post(&mfs_url).send().await?;
        log::info!("Copied IPFS dir to MFS: {}", mfs_path);

        log::info!("Root CID: {} ({}:/ipfs/{})", root, self.config.gateway_url, root);

        Ok((root, cid_by_oth_address))
    }
}