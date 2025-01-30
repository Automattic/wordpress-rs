use x509_cert::der::Decode;
use x509_cert::ext::pkix::name::DirectoryString;
use x509_cert::ext::pkix::name::GeneralName::DnsName;
use x509_cert::ext::pkix::SubjectAltName;
use x509_cert::Certificate;

// Parse a DER-encoded certificate into a Struct we can use to get better
// information about a site's SSL certificate.
//
// If this returns `None`, we weren't able to parse the certificate
#[uniffi::export]
pub fn parse_certificate(data: &[u8]) -> Option<SSLCertificateInfo> {
    let Ok(certificate) = Certificate::from_der(data) else { return None };

    let Some(subject_common_name) = get_common_name(certificate.tbs_certificate().subject()) else { return None };
    let Some(issuer_common_name) = get_common_name(certificate.tbs_certificate().issuer()) else { return None };

    Some(SSLCertificateInfo {
        common_name: subject_common_name,
        alternative_names: get_alternative_names(certificate.tbs_certificate()),
        issuer: SSLCertificateIssuer {
            common_name: issuer_common_name,
            organization: get_organization(certificate.tbs_certificate().issuer()),
            country: get_country(certificate.tbs_certificate().issuer()),
        },
    })
}

pub fn get_common_name(name: &x509_cert::name::Name) -> Option<String> {
    let Ok(Some(cn)) = name.common_name() else {
        return None;
    };
    Some(<DirectoryString as AsRef<str>>::as_ref(&cn).to_string())
}

pub fn get_organization(name: &x509_cert::name::Name) -> Option<String> {
    let Ok(Some(org)) = name.organization() else {
        return None;
    };
    Some(<DirectoryString as AsRef<str>>::as_ref(&org).to_string())
}

pub fn get_country(name: &x509_cert::name::Name) -> Option<String> {
    let Ok(Some(country)) = name.country() else {
        return None;
    };
    Some(<_ as AsRef<str>>::as_ref(&country).to_string())
}

pub fn get_alternative_names(cert: &x509_cert::certificate::TbsCertificateInner) -> Vec<String> {
    let Ok(Some((critical, alternative_names))) = cert.get_extension::<SubjectAltName>() else {
        return vec![];
    };
    alternative_names
        .0
        .into_iter()
        .map(|name| match name {
            DnsName(string) => string.to_string(),
            // TODO: I thought I read that LetsEncrypt would be offering certificates for IP addresses, if so we might need to support that GeneralName variant
            _ => todo!(), // TODO: Do we need to do anything here?
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SSLCertificateInfo {
    // The domain this certificate is valid for (or the signer's name, if this is an intermediate or root certificate)
    pub common_name: String,

    // Other domains this certificate is valid for
    pub alternative_names: Vec<String>,

    // Information about whomever signed this certificate
    pub issuer: SSLCertificateIssuer,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct SSLCertificateIssuer {
    pub common_name: String,
    pub organization: Option<String>,
    pub country: Option<String>,
}
