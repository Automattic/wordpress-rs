import CryptoKit
import Foundation

let count = 15
print("Attempting \(count) concurrent SecureEnclave.P256.KeyAgreement.PrivateKey() calls...")
print("This simulates what happens when Swift Testing runs ~15 SecureEnclave")
print("tests concurrently, each creating its own key.")
print("")

let group = DispatchGroup()
let queue = DispatchQueue(label: "se-test", attributes: .concurrent)
let lock = NSLock()
var results: [(Int, String)] = []

for i in 0..<count {
    group.enter()
    queue.async {
        let start = Date()
        do {
            let key = try SecureEnclave.P256.KeyAgreement.PrivateKey()
            let elapsed = Date().timeIntervalSince(start)
            let pubkey = key.publicKey.x963Representation.prefix(8).map {
                String(format: "%02x", $0)
            }.joined()
            lock.lock()
            results.append((i, String(format: "OK  (%.2fs) key=%s...", elapsed, pubkey)))
            lock.unlock()
        } catch {
            let elapsed = Date().timeIntervalSince(start)
            lock.lock()
            results.append((i, String(format: "ERR (%.2fs) %s", elapsed, "\(error)")))
            lock.unlock()
        }
        group.leave()
    }
}

let timeout: DispatchTimeoutResult = group.wait(timeout: .now() + .seconds(30))

lock.lock()
let finalResults = results.sorted { lhs, rhs in lhs.0 < rhs.0 }
lock.unlock()

for (i, msg) in finalResults {
    print("  [\(i)] \(msg)")
}

if timeout == .timedOut {
    let completed = finalResults.count
    print("")
    print("HANG: Only \(completed)/\(count) key creations completed within 30 seconds.")
    print("This confirms that concurrent Secure Enclave key creation deadlocks.")
    exit(1)
} else {
    print("")
    print("SUCCESS: All \(count) concurrent key creations completed.")
}
