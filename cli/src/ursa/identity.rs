use libp2p::identity::Keypair;

use std::{fs::{self, File}, io::{prelude::*, Result}, path::{Path}};
use std::fs::create_dir_all;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tracing::error;

pub trait Identity {
    fn keypair(&self) -> Keypair;
    fn encode_pem(&self) -> String;
    fn save(&self, path: &PathBuf) -> Result<()>;
    fn load(path: &PathBuf) -> Result<Self> where Self: Sized;
}

impl Identity for Keypair {
    fn keypair(&self) -> Keypair {
        self.clone()
    }

    fn encode_pem(&self) -> String {
        let pem_data = match self {
            Keypair::Ed25519(keypair) => {
                {
                    // todo(oz): A bit static, find a lib that encodes this properly
                    let key = keypair.encode();
                    // ASN.1 header id-ed25519
                    let mut buf: Vec<u8> = vec![0x30, 0x53, 0x02, 0x01, 0x01, 0x30, 0x05, 0x06, 0x03, 0x2B, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20];
                    // extend with secret key
                    buf.extend(key[..32].iter());
                    // extend with pubkey separator
                    buf.extend([0xA1, 0x23, 0x03, 0x21, 0x00].iter());
                    // extend with public key
                    buf.extend(key[32..].iter());

                    pem::Pem {
                        tag: "PRIVATE KEY".to_string(),
                        contents: buf,
                    }
                }
            }
            _ => panic!("Unsupported key type"),
        };

        pem::encode(&pem_data)
    }

    fn save(&self, path: &PathBuf) -> Result<()> {
        let pem = self.encode_pem();
        create_dir_all(path.parent().unwrap())?;
        let mut file = File::create(path)?;
        file.write_all(pem.as_bytes())?;
        file.sync_all()?;
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
        Ok(())
    }

    fn load(path: &PathBuf) -> Result<Self> where Self: Sized {
        // read the file
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let parsed = pem::parse(contents).unwrap();
        let keypair = match parsed.tag.as_str() {
            // PEM encoded ed25519 key
            "PRIVATE KEY" => {
                if parsed.contents.len() != 85 {
                    error!("Invalid ed25519 pkcs#8 v2 key length (is the encoding correct?)");
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid ed25519 key length"));
                }

                let mut buf= [0; 64];
                // private key - offset 16; 32bytes long
                buf[..32].copy_from_slice(&parsed.contents[16..48]);
                // public key - offset 53; 32bytes long
                buf[32..].copy_from_slice(&parsed.contents[53..]);

                Keypair::Ed25519(libp2p::identity::ed25519::Keypair::decode(buf.as_mut()).unwrap())
            }
            _ => panic!("Unsupported key type"),
        };

        Ok(keypair)
    }
}

pub struct IdentityManager<I: Identity> {
    name: String,
    identity: I,
    data_dir: PathBuf,
}

impl Default for IdentityManager<Keypair> {
    fn default() -> Self {
        Self {
            name: "random".to_string(),
            identity: Keypair::generate_ed25519(),
            data_dir: PathBuf::from("data"),
        }
    }
}

impl IdentityManager<Keypair> {
    /// Create a new identity manager with the default id, or create it if it doesn't exist
    pub fn new(data_dir: PathBuf) -> Self {
        let name = "default".to_string();
        let mut path = Path::new(&data_dir).join(&name);
        path.set_extension("pem");
        if path.exists() {
            Self::load(name, data_dir).unwrap()
        } else {
            let im = Self {
                name,
                data_dir,
                ..Self::default()
            };
            im.identity.save(&path).unwrap();
            im
        }
    }
    
    pub fn random(data_dir: PathBuf) -> Self {
        let name = "random".to_string();
        let path = Path::new(&data_dir).join("random.pem");
        let im = Self {
            name,
            data_dir,
            ..Self::default()
        };
        im.identity.save(&path).unwrap();
        im
    }

    /// Load a known identity
    pub fn load<S: Into<String>>(name: S, data_dir: PathBuf) -> Option<Self> {
        let name = name.into();

        let mut path = data_dir.join(&name);
        path.set_extension("pem");

        let identity = match Keypair::load(&path) {
            Ok(identity) => identity,
            Err(e) => {
                error!("Failed to load identity: {}", e);
                return None;
            }
        };

        Some(IdentityManager {
            name,
            identity,
            data_dir,
        })
    }

    /// Save the current identity
    pub fn save(&self) -> Result<()> {
        let mut path = self.data_dir.join(&self.name);
        path.set_extension("pem");
        self.identity.save(&path)
    }

    pub fn current(&self) -> Keypair {
        self.identity.clone()
    }
}
