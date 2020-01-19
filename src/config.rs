use serde_derive::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize, Clone)]
pub struct ConsensusConfig {
    pub grpc_server_port: Option<u16>,
    pub utxo_grpc_port: Option<u16>,
    pub network_grpc_port: Option<u16>,
    pub privkey_path: String,
}

impl ConsensusConfig {
    pub fn new(path: &str) -> Self {
        let mut buffer = String::new();
        File::open(path)
            .and_then(|mut f| f.read_to_string(&mut buffer))
            .unwrap_or_else(|err| panic!("Error while loading config: [{}]", err));
        toml::from_str::<ConsensusConfig>(&buffer).expect("Error while parsing config")
    }
}

#[cfg(test)]
mod tests {
    use super::ConsensusConfig;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn basic_test() {
        let toml_str = r#"
        privkey_path = "0_privkey"
        "#;

        let mut tmp_file: NamedTempFile = NamedTempFile::new().unwrap();
        tmp_file.write_all(toml_str.as_bytes()).unwrap();
        let path = tmp_file.path().to_str().unwrap();
        let config = ConsensusConfig::new(path);

        assert_eq!(config.privkey_path, "0_privkey".to_owned());
    }

    #[test]
    fn options_test() {
        let toml_str = r#"
        grpc_server_port = 5000
        utxo_grpc_port = 5002
        network_grpc_port = 5003
        privkey_path = "0_privkey"
        "#;

        let mut tmp_file: NamedTempFile = NamedTempFile::new().unwrap();
        tmp_file.write_all(toml_str.as_bytes()).unwrap();
        let path = tmp_file.path().to_str().unwrap();
        let config = ConsensusConfig::new(path);

        assert_eq!(config.privkey_path, "0_privkey".to_owned());
        assert_eq!(config.grpc_server_port, Some(5000));
        assert_eq!(config.utxo_grpc_port, Some(5002));
        assert_eq!(config.network_grpc_port, Some(5003));
    }
}
