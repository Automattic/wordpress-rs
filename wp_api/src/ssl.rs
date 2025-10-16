use std::sync::Arc;
use x509_cert::{
    Certificate,
    der::Decode,
    ext::pkix::{SubjectAltName, name::GeneralName::DnsName},
};

use crate::date::WpGmtDateTime;

// Parse a DER-encoded certificate into a Struct we can use to get better
// information about a site's SSL certificate.
//
// If this returns `None`, we weren't able to parse the certificate
#[uniffi::export]
pub fn parse_certificate(data: &[u8]) -> Option<Arc<SslCertificateInfo>> {
    let certificate = Certificate::from_der(data).ok()?;
    let certificate: &x509_cert::certificate::TbsCertificateInner = certificate.tbs_certificate();

    Some(
        SslCertificateInfo {
            valid_at: certificate.validity().not_before.into(),
            expires_at: certificate.validity().not_after.into(),
            common_name: extract_data_as_string(certificate.subject().common_name())?,
            alternative_names: extract_alternative_names(certificate),
            issuer: SSLCertificateIssuer {
                common_name: extract_data_as_string(certificate.issuer().common_name())?,
                organization: extract_data_as_string(certificate.issuer().organization()),
                country: extract_data_as_string(certificate.issuer().country()),
            },
        }
        .into(),
    )
}

fn extract_data_as_string(data: x509_cert::der::Result<Option<impl AsRef<str>>>) -> Option<String> {
    data.ok().flatten().map(|s| s.as_ref().to_string())
}

fn extract_alternative_names(cert: &x509_cert::certificate::TbsCertificateInner) -> Vec<String> {
    let Ok(Some((_critical, alternative_names))) = cert.get_extension::<SubjectAltName>() else {
        return vec![];
    };
    alternative_names
        .0
        .into_iter()
        .flat_map(|name| match name {
            DnsName(string) => Some(string.to_string()),
            // Future Thing: I thought I read that LetsEncrypt would be
            // offering certificates for IP addresses, if so we might need to
            // support that GeneralName variant at some point?
            _ => None,
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq, Hash, uniffi::Object)]
#[uniffi::export(Eq, Hash)]
pub struct SslCertificateInfo {
    /// The domain this certificate is valid for (or the signer's name, if this is an intermediate or root certificate)
    pub common_name: String,
    /// Other domains this certificate is valid for
    pub alternative_names: Vec<String>,
    /// Information about whomever signed this certificate
    pub issuer: SSLCertificateIssuer,
    /// The date this certificate was issued
    pub valid_at: WpGmtDateTime,
    /// The date this certificate expires
    pub expires_at: WpGmtDateTime,
}

#[uniffi::export]
impl SslCertificateInfo {
    fn common_name(&self) -> String {
        self.common_name.clone()
    }

    fn alternative_names(&self) -> Vec<String> {
        self.alternative_names.clone()
    }

    fn issuer(&self) -> SSLCertificateIssuer {
        self.issuer.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, uniffi::Record)]
pub struct SSLCertificateIssuer {
    pub common_name: String,
    pub organization: Option<String>,
    pub country: Option<String>,
}

impl From<x509_cert::time::Time> for WpGmtDateTime {
    fn from(date_time: x509_cert::time::Time) -> Self {
        WpGmtDateTime::from_timestamp(date_time.to_unix_duration().as_secs() as i64)
    }
}
