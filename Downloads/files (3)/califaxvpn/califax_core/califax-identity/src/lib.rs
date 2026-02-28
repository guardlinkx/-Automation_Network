pub mod did;
pub mod zk_proof;
pub mod canary;
pub mod access;
pub mod error;

pub use error::IdentityError;
pub use did::{DidDocument, DidManager};
pub use zk_proof::{ZkProof, ZkVerifier};
pub use canary::{WarrantCanary, CanaryStatus};
pub use access::{AccessPolicy, AccessDecision, ZeroTrustEngine};
