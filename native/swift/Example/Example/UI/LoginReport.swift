import Foundation
import SwiftUI
import WordPressAPI
import AuthenticationServices
import WordPressAPIInternal

struct AutoDiscoveryStepView: View {
    let label: String

    let successIcon = "checkmark.circle"
    let failureIcon = "exclamationmark.circle"

    let isSuccess: Bool

    var body: some View {
        VStack(alignment: .leading) {
            HStack(alignment: .firstTextBaseline) {
                Image(systemName: isSuccess ? successIcon : failureIcon)
                    .font(.title)
                Text(label).font(.title)
                Spacer()
            }
        }.padding(.horizontal)
    }
}

struct AutoDiscoveryErrorView: View {
    let errorMessage: String

    var body: some View {
        Text(errorMessage)
    }
}

struct AutodiscoveryReportView: View {
    let report: AutoDiscoveryResult?

    var body: some View {
        if let report {
            if let success = report.successfulAttempt {
                AutodiscoveryResultView(attempt: success)
            } else if let success = report.autoHttpsAttempt {
                AutodiscoveryResultView(attempt: success)
            } else {
                AutodiscoveryResultView(attempt: report.userInputAttempt)
            }
        } else {
            ProgressView()
        }
    }
}

struct AutodiscoveryResultView: View {
    let attempt: AutoDiscoveryAttemptResult

    var body: some View {
        if let url = attempt.domainWithSubdomain {
            Text(url)
        }

        AutoDiscoveryStepView(label: "Site Connection", isSuccess: attempt.couldConnectToUrl)
        AutoDiscoveryStepView(label: "Can connect using HTTPS", isSuccess: attempt.couldUseHttps)
        AutoDiscoveryStepView(label: "Supports JSON ", isSuccess: attempt.foundApiRoot)
        AutoDiscoveryStepView(label: "Found authentication URL ", isSuccess: attempt.foundAuthenticationUrl)
    }
}

#Preview("Live data") {

    struct AsyncTestView: View {

        @State var report: AutoDiscoveryResult?

        private let loginApi = WordPressLoginClient(requestExecutor: URLSession.shared)

        var body: some View {
            AutodiscoveryReportView(report: report)
                .task {               
                    let result = await loginApi.autodiscoveryResult(forSite: "http://optional-https.wpmt.co")
                    self.report = result
                }
        }
    }

    return AsyncTestView()
}
