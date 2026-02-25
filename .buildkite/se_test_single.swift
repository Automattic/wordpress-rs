import CryptoKit
import Foundation

print("SecureEnclave.isAvailable: \(SecureEnclave.isAvailable)")

guard SecureEnclave.isAvailable else {
    print("Secure Enclave is not available on this machine.")
    exit(0)
}

print("Attempting single SecureEnclave.P256.KeyAgreement.PrivateKey()...")

let done = DispatchSemaphore(value: 0)
var succeeded = false

DispatchQueue.global().async {
    do {
        let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
        print("KEY_CREATED: \(key.publicKey.x963Representation.base64EncodedString())")
        succeeded = true
    } catch {
        print("ERROR: \(error)")
    }
    done.signal()
}

if done.wait(timeout: .now() + .seconds(10)) == .timedOut {
    print("HANG: Single key creation did not complete within 10 seconds.")
    exit(1)
} else if succeeded {
    print("SUCCESS: Single key creation works.")
} else {
    print("FAILED: Single key creation threw an error.")
    exit(1)
}
