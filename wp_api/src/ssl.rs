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
pub fn parse_certificate(data: Vec<u8>) -> Option<Arc<SslCertificateInfo>> {
    let certificate = Certificate::from_der(&data).ok()?;
    let certificate: &x509_cert::certificate::TbsCertificateInner = certificate.tbs_certificate();

    Some(
        SslCertificateInfo {
            valid_at: certificate.validity().not_before.into(),
            expires_at: certificate.validity().not_after.into(),
            // A missing Common Name must not fail the whole parse: modern
            // (SAN-only) certificates omit the subject CN entirely, and CA/Browser
            // Forum direction is to keep doing so. The identities then live in
            // `alternative_names`. See `presented_hostnames`.
            common_name: extract_data_as_string(certificate.subject().common_name()),
            alternative_names: extract_alternative_names(certificate),
            issuer: SSLCertificateIssuer {
                // Likewise optional — a self-signed SAN-only certificate has no
                // CN on its issuer either.
                common_name: extract_data_as_string(certificate.issuer().common_name()),
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
    /// The domain this certificate is valid for (or the signer's name, if this is
    /// an intermediate or root certificate).
    ///
    /// `None` for a SAN-only certificate that omits the subject Common Name — a
    /// certificate is still valid without it, with its identities carried in
    /// `alternative_names`. Prefer `presented_hostnames` over reading this
    /// directly when reporting which names a certificate covers.
    pub common_name: Option<String>,
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
    fn common_name(&self) -> Option<String> {
        self.common_name.clone()
    }

    fn alternative_names(&self) -> Vec<String> {
        self.alternative_names.clone()
    }

    fn issuer(&self) -> SSLCertificateIssuer {
        self.issuer.clone()
    }

    /// Every hostname this certificate presents: the subject Common Name (when
    /// present) followed by the Subject Alternative Names, de-duplicated and in
    /// that order.
    ///
    /// This is what a client should compare the requested host against, and what
    /// belongs in a `CertificateNotValidForName` error's `presented_hostnames` —
    /// it stays correct for SAN-only certificates that have no Common Name, where
    /// reading `common_name` alone would report nothing.
    fn presented_hostnames(&self) -> Vec<String> {
        let mut hostnames: Vec<String> = Vec::new();
        if let Some(common_name) = &self.common_name {
            hostnames.push(common_name.clone());
        }
        for name in &self.alternative_names {
            if !hostnames.contains(name) {
                hostnames.push(name.clone());
            }
        }
        hostnames
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, uniffi::Record)]
pub struct SSLCertificateIssuer {
    /// `None` when the issuer's Distinguished Name carries no Common Name (e.g. a
    /// self-signed SAN-only certificate).
    pub common_name: Option<String>,
    pub organization: Option<String>,
    pub country: Option<String>,
}

impl From<x509_cert::time::Time> for WpGmtDateTime {
    fn from(date_time: x509_cert::time::Time) -> Self {
        WpGmtDateTime::from_unchecked_timestamp(date_time.to_unix_duration().as_secs() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::prelude::{BASE64_STANDARD, Engine as _};

    // A self-signed leaf with `subject=CN=example.com` and
    // `subjectAltName=DNS:example.com,DNS:www.example.com`. Regenerate with:
    //   openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
    //     -keyout /dev/null -days 3650 -subj "/CN=example.com" \
    //     -addext "subjectAltName=DNS:example.com,DNS:www.example.com" | \
    //     openssl x509 -outform DER | base64
    const CERT_WITH_CN: &str = "MIIBqjCCAVCgAwIBAgIUYQuOxfGlV8vImCcwtg+1ImHKAj4wCgYIKoZIzj0EAwIwFjEUMBIGA1UEAwwLZXhhbXBsZS5jb20wHhcNMjYwODA3MjAyOTI2WhcNMzYwODA0MjAyOTI2WjAWMRQwEgYDVQQDDAtleGFtcGxlLmNvbTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABG6xrmcXer5GzTRQE6dhstbLIpd18jNatd7RNJjntzRrxxuwEF6QcBdICfaKyjsVcVSD4+u5aNoAsZtsTOq0zd+jfDB6MB0GA1UdDgQWBBRm6UZIQA11JKjjPpJcplBNCvsi5zAfBgNVHSMEGDAWgBRm6UZIQA11JKjjPpJcplBNCvsi5zAPBgNVHRMBAf8EBTADAQH/MCcGA1UdEQQgMB6CC2V4YW1wbGUuY29tgg93d3cuZXhhbXBsZS5jb20wCgYIKoZIzj0EAwIDSAAwRQIgZA3IT28oIsSBnhI4eU7PyQEAaQxIL2A1a0ns/SJKa98CIQDJ8sjtvUWAsOfgaeOarapqxLq+2Jy7Y4G4bgF7Ga02bQ==";

    // A self-signed SAN-only leaf: the subject and issuer carry only
    // `O=Test Org` (no Common Name), with
    // `subjectAltName=DNS:sanonly.example.com,DNS:alt.example.com`. This is the
    // shape that used to fail the whole parse. Regenerate with:
    //   openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
    //     -keyout /dev/null -days 3650 -subj "/O=Test Org" \
    //     -addext "subjectAltName=DNS:sanonly.example.com,DNS:alt.example.com" | \
    //     openssl x509 -outform DER | base64
    const CERT_WITHOUT_CN: &str = "MIIBrTCCAVSgAwIBAgIUBL104uCOFFWUqBIxVSKRRGH9ZlYwCgYIKoZIzj0EAwIwEzERMA8GA1UECgwIVGVzdCBPcmcwHhcNMjYwODA3MjAyOTI2WhcNMzYwODA0MjAyOTI2WjATMREwDwYDVQQKDAhUZXN0IE9yZzBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABHKpPiIYwCQTsKZ65ClHdTJheu/Wxgodq8pwTP18PtosOLgImbrGZDGE1qtjPJnV8IAnl+0PFBtZmEilucgocs+jgYUwgYIwHQYDVR0OBBYEFNirUP90py0I+xibHJ+72Oh/sUImMB8GA1UdIwQYMBaAFNirUP90py0I+xibHJ+72Oh/sUImMA8GA1UdEwEB/wQFMAMBAf8wLwYDVR0RBCgwJoITc2Fub25seS5leGFtcGxlLmNvbYIPYWx0LmV4YW1wbGUuY29tMAoGCCqGSM49BAMCA0cAMEQCIBoKDl+hLFP2FDhBg7xaf4TMe1FQHdLYJj3WYQaKiyMsAiAEEKmc8R2v3tup78TfMRKmWdilo0NtR3XrNGBBwnqgkA==";

    fn der(base64: &str) -> Vec<u8> {
        BASE64_STANDARD
            .decode(base64)
            .expect("valid base64 test data")
    }

    #[test]
    fn parses_certificate_with_common_name() {
        let cert = parse_certificate(der(CERT_WITH_CN)).expect("certificate should parse");
        assert_eq!(cert.common_name, Some("example.com".to_string()));
        assert_eq!(cert.alternative_names, ["example.com", "www.example.com"]);
        assert_eq!(cert.issuer.common_name, Some("example.com".to_string()));
    }

    #[test]
    fn parses_san_only_certificate_without_common_name() {
        // The regression: a SAN-only leaf (and, being self-signed, a CN-less
        // issuer too) must still parse rather than collapsing to `None`.
        let cert =
            parse_certificate(der(CERT_WITHOUT_CN)).expect("SAN-only certificate should parse");
        assert_eq!(cert.common_name, None);
        assert_eq!(
            cert.alternative_names,
            ["sanonly.example.com", "alt.example.com"]
        );
        assert_eq!(cert.issuer.common_name, None);
        assert_eq!(cert.issuer.organization, Some("Test Org".to_string()));
    }

    #[test]
    fn presented_hostnames_prepend_common_name_and_dedupe() {
        // CN present and also repeated in the SANs: the CN leads, and the
        // duplicate SAN is dropped.
        let cert = parse_certificate(der(CERT_WITH_CN)).expect("certificate with CN should parse");
        assert_eq!(
            cert.presented_hostnames(),
            ["example.com", "www.example.com"]
        );

        // No CN: the presented hostnames are exactly the SANs, so a client still
        // has something to compare the requested host against.
        let san_only =
            parse_certificate(der(CERT_WITHOUT_CN)).expect("SAN-only certificate should parse");
        assert_eq!(
            san_only.presented_hostnames(),
            ["sanonly.example.com", "alt.example.com"]
        );
    }

    #[test]
    fn returns_none_for_non_certificate_input() {
        assert!(parse_certificate(b"not a certificate".to_vec()).is_none());
    }

    // A SAN-only leaf whose subject carries only `O=WordPress-rs Test` (no Common
    // Name), signed by a CA whose own CN *is* present (`wordpress-rs Test CA`).
    // This is the #1508 shape: a CN-less leaf whose issuer/intermediate CN must
    // never be reported as a presented hostname. Regenerate with:
    //   openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
    //     -keyout ca.key -out ca.crt -days 3650 -subj "/CN=wordpress-rs Test CA"
    //   openssl req -newkey ec -pkeyopt ec_paramgen_curve:prime256v1 -nodes \
    //     -keyout leaf.key -out leaf.csr -subj "/O=WordPress-rs Test"
    //   printf "subjectAltName=DNS:san-only.example,DNS:alt.example\n" > san.ext
    //   openssl x509 -req -in leaf.csr -CA ca.crt -CAkey ca.key -CAcreateserial \
    //     -days 3650 -out leaf.crt -extfile san.ext
    //   openssl x509 -in leaf.crt -outform DER | base64
    const CERT_WITHOUT_CN_CA_SIGNED: &str = "MIIBqTCCAU+gAwIBAgIUJ9rF7d+OWjcD0KaW2TszFnszkiAwCgYIKoZIzj0EAwIwHzEdMBsGA1UEAwwUd29yZHByZXNzLXJzIFRlc3QgQ0EwHhcNMjYwODA3MjM0MzQyWhcNMzYwODA0MjM0MzQyWjAcMRowGAYDVQQKDBFXb3JkUHJlc3MtcnMgVGVzdDBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABCJTFadVycou0hYCdrkKKUv9ohMRkj4fpySi+hefIgjEc7HsJc1KgcUmp+OEikPy8bC6PUiJGgA0+qhSMnRWr1CjbDBqMCgGA1UdEQQhMB+CEHNhbi1vbmx5LmV4YW1wbGWCC2FsdC5leGFtcGxlMB0GA1UdDgQWBBQmwdJKGZEt4zkXhR/zS2CTb47S+zAfBgNVHSMEGDAWgBRgJHQjodqqB0/FGnZAbh30gzuLEjAKBggqhkjOPQQDAgNIADBFAiEAp5826iLytnlnI0Z6MDxr7u7nhwuCNJG0+uuVwPrinBwCIE1qPtOJCXxrtWUTmf03nDzx4MzzftDM4eV5xI++7fd9";

    #[test]
    fn ca_signed_san_only_certificate_reports_sans_not_issuer_cn() {
        // Regression for #1508: the leaf has no Common Name, so before the fix it
        // failed to parse and the executors fell back to the issuer/intermediate
        // CA's CN. The presented hostnames must be the leaf's SANs — never the
        // issuer CN, which is present here but must not appear below.
        let cert = parse_certificate(der(CERT_WITHOUT_CN_CA_SIGNED))
            .expect("CA-signed SAN-only certificate should parse");
        assert_eq!(cert.common_name, None);
        assert_eq!(
            cert.issuer.common_name,
            Some("wordpress-rs Test CA".to_string())
        );
        assert_eq!(
            cert.presented_hostnames(),
            ["san-only.example", "alt.example"]
        );
    }
}
