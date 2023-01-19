use libp2p::identity::{ed25519, Keypair};

use libp2p::PeerId;
use std::fs::create_dir_all;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::{
    fs::{self, File},
    io::{prelude::*, Result},
    path::PathBuf,
};
use tracing::{error, info};

pub trait Identity {
    fn id(&self) -> PeerId;
    fn keypair(&self) -> Keypair;
    fn encode_pem(&self) -> String;
    fn save(&self, path: &Path) -> Result<()>;
    fn load(path: &Path) -> Result<Self>
    where
        Self: Sized;
}

impl Identity for Keypair {
    fn id(&self) -> PeerId {
        PeerId::from(self.public())
    }

    fn keypair(&self) -> Keypair {
        self.clone()
    }

    fn encode_pem(&self) -> String {
        let pem_data = match self {
            Keypair::Ed25519(keypair) => {
                {
                    // note(oz): This approach is a bit static, find a lib that does this properly
                    // if we ever accept other signature schemes/pem encodings

                    let key = keypair.encode();
                    // ASN.1 header id-ed25519
                    let mut buf: Vec<u8> = vec![
                        0x30, 0x53, 0x02, 0x01, 0x01, 0x30, 0x05, 0x06, 0x03, 0x2B, 0x65, 0x70,
                        0x04, 0x22, 0x04, 0x20,
                    ];
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
        };

        pem::encode(&pem_data)
    }

    fn save(&self, path: &Path) -> Result<()> {
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

    fn load(path: &Path) -> Result<Self>
    where
        Self: Sized,
    {
        // read the file
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let parsed = pem::parse(contents).unwrap();
        let keypair = match parsed.tag.as_str() {
            // PEM encoded ed25519 key
            "PRIVATE KEY" => {
                // private key - offset 16; 32bytes long
                let sk_bytes = parsed.contents[16..48].to_vec();
                let secret = ed25519::SecretKey::from_bytes(sk_bytes).unwrap();
                Keypair::Ed25519(secret.into())
            }
            _ => panic!("Unsupported key type"),
        };

        Ok(keypair)
    }
}

pub struct IdentityManager<I: Identity> {
    pub name: String,
    pub identity: I,
    pub dir: PathBuf,
}

impl Default for IdentityManager<Keypair> {
    fn default() -> Self {
        Self {
            name: "random".to_string(),
            identity: Keypair::generate_ed25519(),
            dir: PathBuf::from(""),
        }
    }
}

impl IdentityManager<Keypair> {
    pub fn random() -> Self {
        Self::default()
    }

    /// Create a new identity with the given name
    pub fn new<S: Into<String>>(name: S, dir: PathBuf) -> Self {
        let name = name.into();
        let mut path = dir.join(&name);
        path.set_extension("pem");

        let im = Self {
            name: name.clone(),
            dir,
            identity: Keypair::generate_ed25519(),
        };
        im.identity.save(&path).unwrap();

        info!("Created identity `{}` ({})", name, im.identity.id());

        im
    }

    /// Load a known identity
    pub fn load<S: Into<String>>(name: S, dir: PathBuf) -> Option<Self> {
        let name = name.into();

        let mut path = dir.join(&name);
        path.set_extension("pem");

        let identity = match Keypair::load(&path) {
            Ok(identity) => identity,
            Err(e) => {
                error!("Failed to load identity `{}`", e);
                return None;
            }
        };

        info!("Loaded identity `{}` ({})", name, identity.id());

        Some(IdentityManager {
            name,
            identity,
            dir,
        })
    }

    /// Load or create a new identity
    pub fn load_or_new<S: Into<String> + Clone>(name: S, dir: PathBuf) -> Self {
        let name = name.into();
        Self::load(name.clone(), dir.clone()).unwrap_or_else(|| Self::new(name, dir))
    }

    pub fn current(&self) -> Keypair {
        self.identity.clone()
    }
}
