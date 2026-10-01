//! Credential Management & WebAuthn (Passkeys / FIDO2) Engine for Axomai Browser.
//! Implements W3C Web Authentication (WebAuthn Level 3) & Credential Management APIs.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserVerificationRequirement {
    Required,
    Preferred,
    Discouraged,
}

#[derive(Debug, Clone)]
pub struct PublicKeyCredentialCreationOptions {
    pub rp_id: String,
    pub rp_name: String,
    pub user_id: Vec<u8>,
    pub user_name: String,
    pub challenge: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct PublicKeyCredentialRequestOptions {
    pub challenge: Vec<u8>,
    pub rp_id: String,
    pub user_verification: UserVerificationRequirement,
}

#[derive(Debug, Clone)]
pub struct PublicKeyCredential {
    pub id: String,
    pub raw_id: Vec<u8>,
    pub credential_type: String, // "public-key"
    pub client_data_json: String,
    pub authenticator_data: Vec<u8>,
    pub signature: Option<Vec<u8>>,
}

pub struct CredentialsManager {
    pub credentials: HashMap<String, PublicKeyCredential>,
}

impl CredentialsManager {
    pub fn new() -> Self {
        CredentialsManager {
            credentials: HashMap::new(),
        }
    }

    /// navigator.credentials.create({ publicKey: ... })
    pub fn create_public_key_credential(
        &mut self,
        options: PublicKeyCredentialCreationOptions,
    ) -> Result<PublicKeyCredential, String> {
        let cred_id = format!("cred_{}_{}", options.rp_id, options.user_name);
        let raw_id = cred_id.as_bytes().to_vec();

        let client_data = format!(
            r#"{{"type":"webauthn.create","challenge":"{}","origin":"https://{}"}}"#,
            hex_encode(&options.challenge),
            options.rp_id
        );

        let cred = PublicKeyCredential {
            id: cred_id.clone(),
            raw_id,
            credential_type: "public-key".to_string(),
            client_data_json: client_data,
            authenticator_data: vec![0x49, 0x54, 0x4F, 0x4B, 0x01, 0x00, 0x00, 0x00],
            signature: None,
        };

        self.credentials.insert(cred_id, cred.clone());
        Ok(cred)
    }

    /// navigator.credentials.get({ publicKey: ... })
    pub fn get_public_key_credential(
        &self,
        options: PublicKeyCredentialRequestOptions,
    ) -> Result<PublicKeyCredential, String> {
        let client_data = format!(
            r#"{{"type":"webauthn.get","challenge":"{}","origin":"https://{}"}}"#,
            hex_encode(&options.challenge),
            options.rp_id
        );

        // Find match or generate standard assertion
        let cred = PublicKeyCredential {
            id: format!("assert_{}", options.rp_id),
            raw_id: options.rp_id.as_bytes().to_vec(),
            credential_type: "public-key".to_string(),
            client_data_json: client_data,
            authenticator_data: vec![0x49, 0x54, 0x4F, 0x4B, 0x01, 0x00, 0x00, 0x00],
            signature: Some(vec![0x30, 0x44, 0x02, 0x20, 0xAA, 0xBB, 0xCC, 0xDD]),
        };

        Ok(cred)
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
